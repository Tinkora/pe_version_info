#!/usr/bin/env python3
"""Exercise the documented CLI workflow from an isolated consumer directory."""

import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pevi", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    return parser.parse_args()


def executable_path(path: Path) -> Path:
    candidate = path.resolve()
    if candidate.is_file():
        return candidate
    windows_candidate = candidate.with_suffix(".exe")
    if windows_candidate.is_file():
        return windows_candidate
    raise FileNotFoundError(f"pevi executable not found: {candidate}")


def invoke(pevi: Path, *arguments: str, expected_code: int = 0) -> dict[str, object]:
    result = subprocess.run(
        [str(pevi), *arguments],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if result.returncode != expected_code:
        raise AssertionError(
            f"pevi {' '.join(arguments)} returned {result.returncode}: "
            f"{result.stderr.strip()} {result.stdout.strip()}"
        )
    return json.loads(result.stdout)


def toml_path(path: Path) -> str:
    return path.resolve().as_posix().replace('"', '\\"')


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    args = parse_args()
    pevi = executable_path(args.pevi)
    fixture = ROOT / "fixtures/pe64_unsigned.exe"

    with tempfile.TemporaryDirectory(prefix="pevi-clean-consumer-") as temporary:
        consumer = Path(temporary)
        consumer_input = consumer / "consumer.exe"
        output = consumer / "consumer-versioned.exe"
        config = consumer / "pevi.toml"
        unsafe_config = consumer / "unsafe.toml"
        shutil.copyfile(fixture, consumer_input)
        original_hash = digest(consumer_input)

        initialized = invoke(pevi, "init", "--output", str(config))
        if not initialized["ok"] or not config.is_file():
            raise AssertionError("pevi init did not create the consumer configuration")

        config.write_text(
            f'''schema_version = 1
input = "{toml_path(consumer_input)}"
output = "{toml_path(output)}"

[policy]
overwrite_output = false
preserve_unknown_strings = true

[version]
file_version = "7.8.9.10"
product_version = "7.8.9.10"
language = "en-US"
code_page = 1200

[version.strings]
ProductName = "Clean Consumer Acceptance"
''',
            encoding="utf-8",
        )

        inspection = invoke(pevi, "inspect", "--input", str(consumer_input), "--format", "json")
        plan = invoke(pevi, "plan", "--config", str(config), "--format", "json")
        applied = invoke(pevi, "apply", "--config", str(config), "--format", "json")
        verified = invoke(
            pevi,
            "verify",
            "--input",
            str(output),
            "--config",
            str(config),
            "--format",
            "json",
        )

        if inspection["data"]["certificate_table_present"]:
            raise AssertionError("the clean-consumer fixture must be unsigned")
        if plan["data"]["icon"] is not None or not plan["data"]["version_requested"]:
            raise AssertionError("plan did not preserve the version-only request")
        if applied["data"]["icon_changed"] or not applied["data"]["version_changed"]:
            raise AssertionError("apply did not preserve the version-only request")
        if not verified["data"]["matches"]:
            raise AssertionError("verify did not match the consumer request")
        version = verified["data"]["inspection"]["version_info"]
        if version["file_version"] != "7.8.9.10" or version["product_version"] != "7.8.9.10":
            raise AssertionError("Windows version numbers did not round-trip")
        strings = version["string_tables"][0]["strings"]
        if strings["ProductName"] != "Clean Consumer Acceptance":
            raise AssertionError("requested ProductName did not round-trip")
        if strings["UnknownFixtureField"] != "preserve-me":
            raise AssertionError("an unknown VERSIONINFO string was not preserved")

        unsafe_config.write_text(
            config.read_text(encoding="utf-8").replace(
                f'output = "{toml_path(output)}"',
                f'output = "{toml_path(consumer_input)}"',
            ),
            encoding="utf-8",
        )
        rejection_codes = []
        for flags in ((), ("--in-place",), ("--confirm-in-place",)):
            rejected = invoke(
                pevi,
                "apply",
                "--config",
                str(unsafe_config),
                "--format",
                "json",
                *flags,
                expected_code=2,
            )
            rejection_codes.append(rejected["errors"][0]["code"])
        if rejection_codes != ["input_output_same"] * 3:
            raise AssertionError(f"unexpected in-place rejection codes: {rejection_codes}")
        if digest(consumer_input) != original_hash:
            raise AssertionError("a rejected in-place request modified the fixture")

        evidence = {
            "schema_version": 1,
            "workflow": ["init", "inspect", "plan", "apply", "verify"],
            "input": {
                "architecture": inspection["data"]["architecture"],
                "kind": inspection["data"]["kind"],
                "sha256": original_hash,
            },
            "output": {
                "file_version": version["file_version"],
                "product_version": version["product_version"],
                "sha256": applied["data"]["output_sha256"],
                "unknown_string_preserved": True,
                "verified": True,
            },
            "safety": {
                "input_unchanged_after_rejections": True,
                "missing_both_in_place_flags_rejected": True,
                "missing_confirm_in_place_rejected": True,
                "missing_in_place_rejected": True,
            },
        }

    args.evidence.parent.mkdir(parents=True, exist_ok=True)
    args.evidence.write_text(
        json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"clean-consumer acceptance passed: {args.evidence}")


if __name__ == "__main__":
    main()
