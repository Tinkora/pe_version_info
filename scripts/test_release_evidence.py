#!/usr/bin/env python3
"""Regression tests for deterministic release evidence generation."""

import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
GENERATOR = ROOT / "scripts/generate_release_evidence.py"
LINUX = "x86_64-unknown-linux-gnu"
WINDOWS = "x86_64-pc-windows-msvc"


def package(
    source_root: Path,
    name: str,
    *,
    license_expression: str = "MIT",
    license_file: str | None = None,
    source: str | None = "registry+https://github.com/rust-lang/crates.io-index",
) -> dict[str, object]:
    package_root = source_root / name
    package_root.mkdir(parents=True, exist_ok=True)
    (package_root / "Cargo.toml").write_text("[package]\n", encoding="utf-8")
    return {
        "id": f"{source or 'path+file:///workspace'}#{name}@1.0.0",
        "name": name,
        "version": "1.0.0",
        "license": license_expression,
        "license_file": license_file,
        "manifest_path": str(package_root / "Cargo.toml"),
        "source": source,
    }


def dependency(package_id: str, kind: str | None = None) -> dict[str, object]:
    return {
        "name": package_id.rsplit("#", 1)[-1].split("@", 1)[0],
        "pkg": package_id,
        "dep_kinds": [{"kind": kind, "target": None}],
    }


def metadata(
    packages: list[dict[str, object]],
    dependencies: dict[str, list[dict[str, object]]],
    workspace_members: list[str],
) -> dict[str, object]:
    return {
        "packages": packages,
        "resolve": {
            "nodes": [
                {"id": package["id"], "deps": dependencies.get(str(package["id"]), [])}
                for package in packages
            ]
        },
        "workspace_members": workspace_members,
    }


def run_generator(
    dist: Path,
    metadata_inputs: list[tuple[str, Path]],
) -> subprocess.CompletedProcess[str]:
    command = [
        "python3",
        str(GENERATOR),
        "--dist",
        str(dist),
        "--root-package",
        "pevi_cli",
    ]
    for target, metadata_path in metadata_inputs:
        command.extend(["--metadata", f"{target}={metadata_path}"])
    command.extend(
        [
            "--repository",
            "Tinkora/pe_version_info",
            "--version",
            "v0.1.0-alpha.1",
            "--revision",
            "a" * 40,
            "--created",
            "2026-08-14T00:00:00Z",
        ]
    )
    return subprocess.run(command, text=True, capture_output=True)


class ReleaseEvidenceTests(unittest.TestCase):
    def test_generates_target_scoped_evidence_with_complete_license_texts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            dist = work / "dist"
            dist.mkdir()
            (dist / "pevi-v0.1.0-windows.exe").write_bytes(b"windows")
            (dist / "pevi-v0.1.0-linux").write_bytes(b"linux")

            pevi = package(work / "sources", "pevi_cli", source=None)
            serde = package(
                work / "sources",
                "serde",
                license_expression="MIT OR Apache-2.0",
            )
            linux_api = package(
                work / "sources",
                "linux_api",
                license_expression="BSD-2-Clause OR Apache-2.0",
            )
            windows_api = package(
                work / "sources", "windows_api", license_expression="Zlib"
            )
            build_helper = package(work / "sources", "build_helper")
            assert_cmd = package(work / "sources", "assert_cmd")
            predicates = package(work / "sources", "predicates")
            wait_timeout = package(work / "sources", "wait-timeout")

            (Path(str(serde["manifest_path"])).parent / "LICENSE-MIT").write_text(
                "SERDE MIT LICENSE TEXT\n", encoding="utf-8"
            )
            (Path(str(serde["manifest_path"])).parent / "LICENSE-APACHE").write_text(
                "SERDE APACHE LICENSE TEXT\n", encoding="utf-8"
            )
            (Path(str(linux_api["manifest_path"])).parent / "LICENSE.md").write_text(
                "LINUX BSD LICENSE TEXT\n", encoding="utf-8"
            )
            (Path(str(linux_api["manifest_path"])).parent / "LICENSE-APACHE.md").write_text(
                "LINUX APACHE LICENSE TEXT\n", encoding="utf-8"
            )
            (Path(str(linux_api["manifest_path"])).parent / "NOTICE").write_text(
                "LINUX REQUIRED NOTICE\n", encoding="utf-8"
            )
            (Path(str(windows_api["manifest_path"])).parent / "LICENSE").write_text(
                "WINDOWS ZLIB LICENSE TEXT\n", encoding="utf-8"
            )
            (Path(str(build_helper["manifest_path"])).parent / "LICENSE-MIT").write_text(
                "BUILD HELPER MIT LICENSE TEXT\n", encoding="utf-8"
            )

            packages = [
                pevi,
                serde,
                linux_api,
                windows_api,
                build_helper,
                assert_cmd,
                predicates,
                wait_timeout,
            ]
            common_root_dependencies = [
                dependency(str(serde["id"])),
                dependency(str(build_helper["id"]), "build"),
                dependency(str(assert_cmd["id"]), "dev"),
            ]
            common_dependencies = {
                str(serde["id"]): [],
                str(build_helper["id"]): [],
                str(assert_cmd["id"]): [dependency(str(predicates["id"]))],
                str(predicates["id"]): [dependency(str(wait_timeout["id"]))],
                str(wait_timeout["id"]): [],
            }
            linux_metadata = metadata(
                packages,
                {
                    **common_dependencies,
                    str(pevi["id"]): common_root_dependencies
                    + [dependency(str(linux_api["id"]))],
                    str(linux_api["id"]): [],
                },
                [str(pevi["id"])],
            )
            windows_metadata = metadata(
                packages,
                {
                    **common_dependencies,
                    str(pevi["id"]): common_root_dependencies
                    + [dependency(str(windows_api["id"]))],
                    str(windows_api["id"]): [],
                },
                [str(pevi["id"])],
            )
            linux_metadata_path = work / "linux.metadata.json"
            windows_metadata_path = work / "windows.metadata.json"
            linux_metadata_path.write_text(json.dumps(linux_metadata), encoding="utf-8")
            windows_metadata_path.write_text(json.dumps(windows_metadata), encoding="utf-8")

            result = run_generator(
                dist,
                [(LINUX, linux_metadata_path), (WINDOWS, windows_metadata_path)],
            )
            self.assertEqual(result.returncode, 0, result.stderr)

            checksum_lines = (dist / "SHA256SUMS").read_text(encoding="utf-8").splitlines()
            self.assertEqual(
                checksum_lines,
                [
                    f"{hashlib.sha256(b'linux').hexdigest()}  pevi-v0.1.0-linux",
                    f"{hashlib.sha256(b'windows').hexdigest()}  pevi-v0.1.0-windows.exe",
                ],
            )

            sbom = json.loads((dist / "sbom.spdx.json").read_text(encoding="utf-8"))
            sbom_names = {item["name"] for item in sbom["packages"]}
            self.assertEqual(
                sbom_names,
                {"pevi_cli", "serde", "linux_api", "windows_api", "build_helper"},
            )
            self.assertTrue(
                {"assert_cmd", "predicates", "wait-timeout"}.isdisjoint(sbom_names)
            )

            inventory = json.loads(
                (dist / "license_inventory.json").read_text(encoding="utf-8")
            )
            inventory_by_name = {item["name"]: item for item in inventory["packages"]}
            self.assertEqual(inventory["schema_version"], 2)
            self.assertEqual(inventory["root_package"], "pevi_cli")
            self.assertEqual(inventory["targets"], [WINDOWS, LINUX])
            self.assertEqual(inventory_by_name["serde"]["targets"], [WINDOWS, LINUX])
            self.assertEqual(inventory_by_name["linux_api"]["targets"], [LINUX])
            self.assertEqual(inventory_by_name["windows_api"]["targets"], [WINDOWS])
            self.assertEqual(
                [item["path"] for item in inventory_by_name["serde"]["license_files"]],
                ["LICENSE-APACHE", "LICENSE-MIT"],
            )
            self.assertEqual(
                inventory_by_name["serde"]["license_files"][1]["sha256"],
                hashlib.sha256(b"SERDE MIT LICENSE TEXT\n").hexdigest(),
            )
            self.assertNotIn("/private/", json.dumps(inventory))

            notices = (dist / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")
            for expected in (
                "SERDE MIT LICENSE TEXT",
                "SERDE APACHE LICENSE TEXT",
                "LINUX BSD LICENSE TEXT",
                "LINUX APACHE LICENSE TEXT",
                "LINUX REQUIRED NOTICE",
                "WINDOWS ZLIB LICENSE TEXT",
                "BUILD HELPER MIT LICENSE TEXT",
            ):
                self.assertIn(expected, notices)
            for excluded in ("assert_cmd", "predicates", "wait-timeout"):
                self.assertNotIn(excluded, notices)

    def test_uses_explicit_license_file_and_collects_notice(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            dist = work / "dist"
            dist.mkdir()
            (dist / "pevi").write_bytes(b"binary")
            pevi = package(work / "sources", "pevi_cli", source=None)
            dependency_package = package(
                work / "sources",
                "custom_license",
                license_expression="LicenseRef-Custom",
                license_file="LEGAL.txt",
            )
            package_root = Path(str(dependency_package["manifest_path"])).parent
            (package_root / "LEGAL.txt").write_text(
                "CUSTOM LICENSE BODY\n", encoding="utf-8"
            )
            (package_root / "NOTICE.md").write_text(
                "CUSTOM ATTRIBUTION\n", encoding="utf-8"
            )
            input_metadata = metadata(
                [pevi, dependency_package],
                {
                    str(pevi["id"]): [dependency(str(dependency_package["id"]))],
                    str(dependency_package["id"]): [],
                },
                [str(pevi["id"])],
            )
            metadata_path = work / "metadata.json"
            metadata_path.write_text(json.dumps(input_metadata), encoding="utf-8")

            result = run_generator(dist, [(LINUX, metadata_path)])
            self.assertEqual(result.returncode, 0, result.stderr)
            notices = (dist / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")
            self.assertIn("CUSTOM LICENSE BODY", notices)
            self.assertIn("CUSTOM ATTRIBUTION", notices)

    def test_fails_when_declared_licenses_cannot_be_mapped_to_texts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            dist = work / "dist"
            dist.mkdir()
            (dist / "pevi").write_bytes(b"binary")
            pevi = package(work / "sources", "pevi_cli", source=None)
            ambiguous = package(
                work / "sources", "ambiguous", license_expression="MIT OR Apache-2.0"
            )
            (Path(str(ambiguous["manifest_path"])).parent / "LICENSE").write_text(
                "ONE UNIDENTIFIED LICENSE\n", encoding="utf-8"
            )
            input_metadata = metadata(
                [pevi, ambiguous],
                {
                    str(pevi["id"]): [dependency(str(ambiguous["id"]))],
                    str(ambiguous["id"]): [],
                },
                [str(pevi["id"])],
            )
            metadata_path = work / "metadata.json"
            metadata_path.write_text(json.dumps(input_metadata), encoding="utf-8")

            result = run_generator(dist, [(LINUX, metadata_path)])
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(
                "cannot map declared license 'MIT OR Apache-2.0' to license files",
                result.stderr,
            )
            self.assertFalse((dist / "THIRD_PARTY_NOTICES.md").exists())

    def test_fails_when_a_third_party_package_has_no_license_text(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            dist = work / "dist"
            dist.mkdir()
            (dist / "pevi").write_bytes(b"binary")
            pevi = package(work / "sources", "pevi_cli", source=None)
            missing = package(work / "sources", "missing")
            input_metadata = metadata(
                [pevi, missing],
                {
                    str(pevi["id"]): [dependency(str(missing["id"]))],
                    str(missing["id"]): [],
                },
                [str(pevi["id"])],
            )
            metadata_path = work / "metadata.json"
            metadata_path.write_text(json.dumps(input_metadata), encoding="utf-8")

            result = run_generator(dist, [(LINUX, metadata_path)])
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("missing license text for missing 1.0.0", result.stderr)

    def test_rejects_one_generic_bsd_file_for_multiple_bsd_variants(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            dist = work / "dist"
            dist.mkdir()
            (dist / "pevi").write_bytes(b"binary")
            pevi = package(work / "sources", "pevi_cli", source=None)
            ambiguous = package(
                work / "sources",
                "ambiguous_bsd",
                license_expression="BSD-2-Clause OR BSD-3-Clause",
            )
            (Path(str(ambiguous["manifest_path"])).parent / "LICENSE-BSD").write_text(
                "UNIDENTIFIED BSD LICENSE\n", encoding="utf-8"
            )
            input_metadata = metadata(
                [pevi, ambiguous],
                {
                    str(pevi["id"]): [dependency(str(ambiguous["id"]))],
                    str(ambiguous["id"]): [],
                },
                [str(pevi["id"])],
            )
            metadata_path = work / "metadata.json"
            metadata_path.write_text(json.dumps(input_metadata), encoding="utf-8")

            result = run_generator(dist, [(LINUX, metadata_path)])
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(
                "cannot map declared license 'BSD-2-Clause OR BSD-3-Clause'",
                result.stderr,
            )

    def test_rejects_symlinked_license_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            dist = work / "dist"
            dist.mkdir()
            (dist / "pevi").write_bytes(b"binary")
            pevi = package(work / "sources", "pevi_cli", source=None)
            untrusted = package(work / "sources", "untrusted")
            outside = work / "outside-license"
            outside.write_text("OUTSIDE FILE\n", encoding="utf-8")
            (Path(str(untrusted["manifest_path"])).parent / "LICENSE-MIT").symlink_to(
                outside
            )
            input_metadata = metadata(
                [pevi, untrusted],
                {
                    str(pevi["id"]): [dependency(str(untrusted["id"]))],
                    str(untrusted["id"]): [],
                },
                [str(pevi["id"])],
            )
            metadata_path = work / "metadata.json"
            metadata_path.write_text(json.dumps(input_metadata), encoding="utf-8")

            result = run_generator(dist, [(LINUX, metadata_path)])
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("symlinked license or notice file", result.stderr)

    def test_spdx_ids_remain_unique_after_name_normalization(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            dist = work / "dist"
            dist.mkdir()
            (dist / "pevi").write_bytes(b"binary")
            pevi = package(work / "sources", "pevi_cli", source=None)
            foo_dash = package(work / "sources", "foo-bar")
            foo_underscore = package(work / "sources", "foo_bar")
            for item in (foo_dash, foo_underscore):
                (Path(str(item["manifest_path"])).parent / "LICENSE-MIT").write_text(
                    "MIT LICENSE TEXT\n", encoding="utf-8"
                )
            input_metadata = metadata(
                [pevi, foo_dash, foo_underscore],
                {
                    str(pevi["id"]): [
                        dependency(str(foo_dash["id"])),
                        dependency(str(foo_underscore["id"])),
                    ],
                    str(foo_dash["id"]): [],
                    str(foo_underscore["id"]): [],
                },
                [str(pevi["id"])],
            )
            metadata_path = work / "metadata.json"
            metadata_path.write_text(json.dumps(input_metadata), encoding="utf-8")

            result = run_generator(dist, [(LINUX, metadata_path)])
            self.assertEqual(result.returncode, 0, result.stderr)
            sbom = json.loads((dist / "sbom.spdx.json").read_text(encoding="utf-8"))
            identifiers = [item["SPDXID"] for item in sbom["packages"]]
            self.assertEqual(len(identifiers), len(set(identifiers)))


if __name__ == "__main__":
    unittest.main()
