#!/usr/bin/env python3
"""Generate deterministic checksums, SPDX SBOM, and license evidence."""

import argparse
import hashlib
import json
import re
from pathlib import Path
from urllib.parse import quote


EVIDENCE_FILES = {
    "SHA256SUMS",
    "THIRD_PARTY_NOTICES.md",
    "license_inventory.json",
    "sbom.spdx.json",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dist", type=Path, required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--created", required=True)
    return parser.parse_args()


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def artifact_files(dist: Path) -> list[Path]:
    files = [
        path
        for path in dist.iterdir()
        if path.is_file()
        and not path.is_symlink()
        and path.name not in EVIDENCE_FILES
        and not path.name.endswith(".sha256")
    ]
    if not files:
        raise ValueError("dist contains no release artifacts")
    return sorted(files, key=lambda path: path.name)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def spdx_id(name: str, version: str, suffix: str = "") -> str:
    value = re.sub(r"[^A-Za-z0-9.-]+", "-", f"{name}-{version}{suffix}").strip("-")
    return f"SPDXRef-Package-{value}"


def package_ids(packages: list[dict[str, object]]) -> dict[str, str]:
    grouped: dict[str, list[str]] = {}
    for package in packages:
        base = spdx_id(str(package["name"]), str(package["version"]))
        grouped.setdefault(base, []).append(str(package["id"]))

    result = {}
    for base, ids in grouped.items():
        for package_id in sorted(ids):
            if len(ids) > 1:
                suffix = hashlib.sha256(package_id.encode()).hexdigest()[:12]
                result[package_id] = f"{base}-{suffix}"
            else:
                result[package_id] = base
    return result


def source_label(package: dict[str, object], workspace_members: set[str]) -> str:
    if str(package["id"]) in workspace_members:
        return "workspace"
    source = package.get("source")
    return str(source) if source else "unknown"


def declared_license(package: dict[str, object]) -> str:
    license_expression = package.get("license")
    return str(license_expression) if license_expression else "NOASSERTION"


def build_spdx(
    metadata: dict[str, object],
    repository: str,
    version: str,
    revision: str,
    created: str,
) -> dict[str, object]:
    packages = sorted(
        metadata["packages"],
        key=lambda package: (str(package["name"]), str(package["version"]), str(package["id"])),
    )
    ids = package_ids(packages)
    workspace_members = set(metadata.get("workspace_members", []))
    spdx_packages = []
    for package in packages:
        package_id = str(package["id"])
        name = str(package["name"])
        package_version = str(package["version"])
        spdx_packages.append(
            {
                "SPDXID": ids[package_id],
                "copyrightText": "NOASSERTION",
                "downloadLocation": "NOASSERTION",
                "externalRefs": [
                    {
                        "referenceCategory": "PACKAGE-MANAGER",
                        "referenceLocator": f"pkg:cargo/{quote(name)}@{quote(package_version)}",
                        "referenceType": "purl",
                    }
                ],
                "filesAnalyzed": False,
                "licenseConcluded": "NOASSERTION",
                "licenseDeclared": declared_license(package),
                "name": name,
                "sourceInfo": source_label(package, workspace_members),
                "versionInfo": package_version,
            }
        )

    relationships = []
    resolve = metadata.get("resolve") or {}
    for node in sorted(resolve.get("nodes", []), key=lambda item: str(item["id"])):
        source_id = str(node["id"])
        if source_id not in ids:
            continue
        for dependency in sorted(node.get("deps", []), key=lambda item: str(item["pkg"])):
            dependency_id = str(dependency["pkg"])
            if dependency_id in ids:
                relationships.append(
                    {
                        "relatedSpdxElement": ids[dependency_id],
                        "relationshipType": "DEPENDS_ON",
                        "spdxElementId": ids[source_id],
                    }
                )

    for member in sorted(workspace_members):
        if member in ids:
            relationships.append(
                {
                    "relatedSpdxElement": ids[member],
                    "relationshipType": "DESCRIBES",
                    "spdxElementId": "SPDXRef-DOCUMENT",
                }
            )

    return {
        "SPDXID": "SPDXRef-DOCUMENT",
        "creationInfo": {
            "created": created,
            "creators": ["Tool: Tinkora generate_release_evidence.py"],
        },
        "dataLicense": "CC0-1.0",
        "documentNamespace": f"https://github.com/{repository}/sbom/{quote(version)}/{revision}",
        "name": f"{repository.replace('/', '-')}-{version}",
        "packages": spdx_packages,
        "relationships": relationships,
        "spdxVersion": "SPDX-2.3",
    }


def build_license_inventory(metadata: dict[str, object]) -> dict[str, object]:
    workspace_members = set(metadata.get("workspace_members", []))
    packages = []
    for package in sorted(
        metadata["packages"],
        key=lambda item: (str(item["name"]), str(item["version"]), str(item["id"])),
    ):
        license_file = package.get("license_file")
        packages.append(
            {
                "license": declared_license(package),
                "license_file": Path(str(license_file)).name if license_file else None,
                "name": str(package["name"]),
                "source": source_label(package, workspace_members),
                "version": str(package["version"]),
                "workspace": str(package["id"]) in workspace_members,
            }
        )
    return {"schema_version": 1, "packages": packages}


def write_notices(path: Path, inventory: dict[str, object]) -> None:
    third_party = [package for package in inventory["packages"] if not package["workspace"]]
    lines = [
        "# Third-Party Notices",
        "",
        "This release includes the following Rust dependencies. License expressions are",
        "reported from the locked Cargo package metadata.",
        "",
        "| Package | Version | License |",
        "| --- | --- | --- |",
    ]
    for package in third_party:
        lines.append(f"| {package['name']} | {package['version']} | {package['license']} |")
    lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    args = parse_args()
    if not args.dist.is_dir():
        raise ValueError("dist must be an existing directory")
    metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
    artifacts = artifact_files(args.dist)
    checksums = "".join(f"{sha256(path)}  {path.name}\n" for path in artifacts)
    (args.dist / "SHA256SUMS").write_text(checksums, encoding="ascii")

    inventory = build_license_inventory(metadata)
    write_json(args.dist / "license_inventory.json", inventory)
    write_notices(args.dist / "THIRD_PARTY_NOTICES.md", inventory)
    write_json(
        args.dist / "sbom.spdx.json",
        build_spdx(metadata, args.repository, args.version, args.revision, args.created),
    )


if __name__ == "__main__":
    main()
