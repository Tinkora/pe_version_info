#!/usr/bin/env python3
"""Generate target-scoped checksums, SPDX SBOM, and license evidence."""

import argparse
import hashlib
import json
import re
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import quote


EVIDENCE_FILES = {
    "SHA256SUMS",
    "THIRD_PARTY_NOTICES.md",
    "license_inventory.json",
    "sbom.spdx.json",
}
LICENSE_PREFIXES = ("LICENSE", "LICENCE", "COPYING", "UNLICENSE")
NOTICE_PREFIXES = ("NOTICE",)
EXPRESSION_OPERATORS = {"AND", "OR", "WITH"}
LICENSE_FILE_ALIASES = {
    "0BSD": ("0BSD",),
    "APACHE-2.0": ("APACHE",),
    "BSD-2-CLAUSE": ("BSD-2", "BSD2"),
    "BSD-3-CLAUSE": ("BSD-3", "BSD3"),
    "BSL-1.0": ("BSL", "BOOST"),
    "CC0-1.0": ("CC0",),
    "ISC": ("ISC",),
    "LLVM-EXCEPTION": ("LLVM",),
    "MIT": ("MIT",),
    "MPL-2.0": ("MPL",),
    "UNICODE-3.0": ("UNICODE",),
    "UNLICENSE": ("UNLICENSE",),
    "ZLIB": ("ZLIB",),
}


@dataclass(frozen=True)
class MetadataInput:
    target: str
    path: Path


@dataclass
class DistributionGraph:
    packages: dict[str, dict[str, object]]
    relationships: set[tuple[str, str]]
    root_ids: set[str]
    targets_by_package: dict[str, set[str]]
    workspace_members: set[str]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dist", type=Path, required=True)
    parser.add_argument(
        "--metadata",
        action="append",
        required=True,
        metavar="TARGET=PATH",
        help="Cargo metadata filtered for one released target; repeat per target",
    )
    parser.add_argument("--root-package", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--created", required=True)
    args = parser.parse_args()
    args.metadata = parse_metadata_inputs(args.metadata)
    return args


def parse_metadata_inputs(values: list[str]) -> list[MetadataInput]:
    inputs = []
    targets = set()
    for value in values:
        target, separator, raw_path = value.partition("=")
        if not separator or not target or not raw_path:
            raise ValueError(f"metadata must use TARGET=PATH: {value}")
        if target in targets:
            raise ValueError(f"duplicate metadata target: {target}")
        targets.add(target)
        inputs.append(MetadataInput(target=target, path=Path(raw_path)))
    return sorted(inputs, key=lambda item: item.target)


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


def sha256_bytes(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def distribution_graph(
    metadata_inputs: list[MetadataInput], root_package: str
) -> DistributionGraph:
    packages: dict[str, dict[str, object]] = {}
    relationships: set[tuple[str, str]] = set()
    root_ids: set[str] = set()
    targets_by_package: dict[str, set[str]] = {}
    workspace_members: set[str] = set()

    for metadata_input in metadata_inputs:
        metadata = json.loads(metadata_input.path.read_text(encoding="utf-8"))
        package_by_id = {str(item["id"]): item for item in metadata["packages"]}
        roots = [
            item
            for item in metadata["packages"]
            if item["name"] == root_package
            and str(item["id"]) in set(metadata.get("workspace_members", []))
        ]
        if len(roots) != 1:
            raise ValueError(
                f"expected one workspace package named {root_package!r} for "
                f"{metadata_input.target}, found {len(roots)}"
            )
        root_id = str(roots[0]["id"])
        root_ids.add(root_id)
        nodes = {
            str(node["id"]): node for node in (metadata.get("resolve") or {}).get("nodes", [])
        }
        if root_id not in nodes:
            raise ValueError(
                f"resolve graph for {metadata_input.target} does not contain {root_package!r}"
            )

        selected = set()
        pending = [root_id]
        while pending:
            package_id = pending.pop()
            if package_id in selected:
                continue
            if package_id not in package_by_id or package_id not in nodes:
                raise ValueError(
                    f"incomplete Cargo metadata for {metadata_input.target}: {package_id}"
                )
            selected.add(package_id)
            for dependency_item in nodes[package_id].get("deps", []):
                dependency_id = str(dependency_item["pkg"])
                kinds = dependency_item.get("dep_kinds", [])
                if not any(kind.get("kind") in (None, "build") for kind in kinds):
                    continue
                relationships.add((package_id, dependency_id))
                pending.append(dependency_id)

        for package_id in selected:
            package = package_by_id[package_id]
            existing = packages.get(package_id)
            if existing is not None and package_identity(existing) != package_identity(package):
                raise ValueError(f"conflicting metadata for package {package_id}")
            packages[package_id] = package
            targets_by_package.setdefault(package_id, set()).add(metadata_input.target)
        workspace_members.update(
            package_id
            for package_id in metadata.get("workspace_members", [])
            if package_id in selected
        )

    return DistributionGraph(
        packages=packages,
        relationships=relationships,
        root_ids=root_ids,
        targets_by_package=targets_by_package,
        workspace_members=workspace_members,
    )


def package_identity(package: dict[str, object]) -> tuple[object, ...]:
    return (
        package.get("name"),
        package.get("version"),
        package.get("source"),
        package.get("license"),
        package.get("license_file"),
    )


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


def sorted_packages(graph: DistributionGraph) -> list[dict[str, object]]:
    return sorted(
        graph.packages.values(),
        key=lambda package: (
            str(package["name"]),
            str(package["version"]),
            str(package["id"]),
        ),
    )


def build_spdx(
    graph: DistributionGraph,
    repository: str,
    version: str,
    revision: str,
    created: str,
) -> dict[str, object]:
    packages = sorted_packages(graph)
    ids = package_ids(packages)
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
                "sourceInfo": source_label(package, graph.workspace_members),
                "versionInfo": package_version,
            }
        )

    relationships = [
        {
            "relatedSpdxElement": ids[dependency_id],
            "relationshipType": "DEPENDS_ON",
            "spdxElementId": ids[source_id],
        }
        for source_id, dependency_id in sorted(graph.relationships)
    ]
    relationships.extend(
        {
            "relatedSpdxElement": ids[root_id],
            "relationshipType": "DESCRIBES",
            "spdxElementId": "SPDXRef-DOCUMENT",
        }
        for root_id in sorted(graph.root_ids)
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


def package_root(package: dict[str, object]) -> Path:
    manifest = Path(str(package.get("manifest_path", "")))
    if not manifest.is_file():
        raise ValueError(f"manifest is missing for {package_label(package)}: {manifest}")
    return manifest.resolve().parent


def package_label(package: dict[str, object]) -> str:
    return f"{package['name']} {package['version']}"


def top_level_files(root: Path, prefixes: tuple[str, ...]) -> list[Path]:
    files = sorted(
        (
            path
            for path in root.iterdir()
            if path.is_file() and path.name.upper().startswith(prefixes)
        ),
        key=lambda path: path.name,
    )
    for path in files:
        if path.is_symlink():
            raise ValueError(
                f"symlinked license or notice file is not allowed: {path.name}"
            )
    return files


def expression_identifiers(expression: str) -> list[str]:
    identifiers = re.findall(r"[A-Za-z0-9][A-Za-z0-9.+-]*", expression)
    return [item for item in identifiers if item.upper() not in EXPRESSION_OPERATORS]


def normalized_filename(path: Path) -> str:
    return re.sub(r"[^A-Z0-9]+", "-", path.name.upper()).strip("-")


def is_generic_license_filename(path: Path) -> bool:
    stem = path.name.upper()
    for suffix in (".MD", ".MARKDOWN", ".TXT"):
        if stem.endswith(suffix):
            stem = stem[: -len(suffix)]
            break
    return stem in {"LICENSE", "LICENCE", "COPYING"}


def validate_license_files(
    package: dict[str, object], candidates: list[Path]
) -> list[Path]:
    explicit_license_file = package.get("license_file")
    if explicit_license_file:
        root = package_root(package)
        explicit = Path(str(explicit_license_file))
        if not explicit.is_absolute():
            explicit = root / explicit
        if explicit.is_symlink():
            raise ValueError(
                f"symlinked license or notice file is not allowed: {explicit.name}"
            )
        explicit = explicit.resolve()
        if not explicit.is_file() or not explicit.is_relative_to(root):
            raise ValueError(
                f"invalid license_file for {package_label(package)}: {explicit_license_file}"
            )
        return [explicit]

    expression = package.get("license")
    if not expression:
        raise ValueError(f"missing license metadata for {package_label(package)}")
    identifiers = expression_identifiers(str(expression))
    if not identifiers:
        raise ValueError(f"invalid license expression for {package_label(package)}: {expression}")
    if not candidates:
        raise ValueError(f"missing license text for {package_label(package)}")
    if len(identifiers) == 1 and len(candidates) == 1:
        return candidates

    bsd_identifiers = [identifier for identifier in identifiers if identifier.upper().startswith("BSD-")]
    selected = set()
    unmatched = []
    for identifier in identifiers:
        aliases = LICENSE_FILE_ALIASES.get(identifier.upper(), (identifier.upper(),))
        if identifier in bsd_identifiers and len(bsd_identifiers) == 1:
            aliases += ("BSD",)
        matches = [
            path
            for path in candidates
            if any(alias in normalized_filename(path) for alias in aliases)
        ]
        if not matches:
            unmatched.append(identifier)
        else:
            selected.update(matches)
    unnamed = [
        path
        for path in candidates
        if path not in selected
        and is_generic_license_filename(path)
    ]
    if unmatched:
        if len(unmatched) != 1 or len(unnamed) != 1:
            raise ValueError(
                f"cannot map declared license {str(expression)!r} to license files for "
                f"{package_label(package)}"
            )
        selected.add(unnamed[0])
    return candidates


def evidence_file(path: Path) -> dict[str, str]:
    content = path.read_bytes()
    content.decode("utf-8")
    return {"path": path.name, "sha256": sha256_bytes(content)}


def collect_license_evidence(
    graph: DistributionGraph, root_package: str
) -> tuple[dict[str, object], dict[str, list[Path]]]:
    packages = []
    evidence_paths = {}
    for package in sorted_packages(graph):
        package_id = str(package["id"])
        workspace = package_id in graph.workspace_members
        item: dict[str, object] = {
            "license": declared_license(package),
            "name": str(package["name"]),
            "source": source_label(package, graph.workspace_members),
            "targets": sorted(graph.targets_by_package[package_id]),
            "version": str(package["version"]),
            "workspace": workspace,
        }
        if workspace:
            item["license_files"] = []
            item["notice_files"] = []
        else:
            root = package_root(package)
            license_paths = validate_license_files(
                package, top_level_files(root, LICENSE_PREFIXES)
            )
            notice_paths = top_level_files(root, NOTICE_PREFIXES)
            item["license_files"] = [evidence_file(path) for path in license_paths]
            item["notice_files"] = [evidence_file(path) for path in notice_paths]
            evidence_paths[package_id] = license_paths + notice_paths
        packages.append(item)
    return (
        {
            "root_package": root_package,
            "schema_version": 2,
            "targets": sorted(
                {
                    target
                for targets in graph.targets_by_package.values()
                for target in targets
                }
            ),
            "packages": packages,
        },
        evidence_paths,
    )


def markdown_code_block(content: str) -> list[str]:
    longest = max((len(match) for match in re.findall(r"`+", content)), default=0)
    fence = "`" * max(3, longest + 1)
    return [f"{fence}text", content.rstrip("\n"), fence]


def write_notices(
    path: Path,
    inventory: dict[str, object],
    graph: DistributionGraph,
    evidence_paths: dict[str, list[Path]],
) -> None:
    package_id_by_identity = {
        (str(package["name"]), str(package["version"]), source_label(package, graph.workspace_members)): str(
            package["id"]
        )
        for package in graph.packages.values()
    }
    lines = [
        "# Third-Party Notices",
        "",
        "This file contains the license and notice texts shipped by every third-party",
        "Rust package in the released `pevi_cli` dependency closure. The closure is the",
        "union of the released targets and excludes development-only dependencies.",
        "Workspace components are covered by the repository's root `LICENSE` file.",
        "",
    ]
    third_party = [item for item in inventory["packages"] if not item["workspace"]]
    for item in third_party:
        package_id = package_id_by_identity[
            (str(item["name"]), str(item["version"]), str(item["source"]))
        ]
        lines.extend(
            [
                f"## {item['name']} {item['version']}",
                "",
                f"- Declared license: `{item['license']}`",
                f"- Released targets: {', '.join(f'`{target}`' for target in item['targets'])}",
                "",
            ]
        )
        license_names = {entry["path"] for entry in item["license_files"]}
        for evidence_path in evidence_paths[package_id]:
            kind = "License" if evidence_path.name in license_names else "Notice"
            lines.extend([f"### {kind}: {evidence_path.name}", ""])
            lines.extend(markdown_code_block(evidence_path.read_text(encoding="utf-8")))
            lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    args = parse_args()
    if not args.dist.is_dir():
        raise ValueError("dist must be an existing directory")
    artifacts = artifact_files(args.dist)
    graph = distribution_graph(args.metadata, args.root_package)
    inventory, evidence_paths = collect_license_evidence(graph, args.root_package)
    sbom = build_spdx(
        graph, args.repository, args.version, args.revision, args.created
    )

    checksums = "".join(f"{sha256(path)}  {path.name}\n" for path in artifacts)
    (args.dist / "SHA256SUMS").write_text(checksums, encoding="ascii")
    write_json(args.dist / "license_inventory.json", inventory)
    write_notices(
        args.dist / "THIRD_PARTY_NOTICES.md", inventory, graph, evidence_paths
    )
    write_json(args.dist / "sbom.spdx.json", sbom)


if __name__ == "__main__":
    main()
