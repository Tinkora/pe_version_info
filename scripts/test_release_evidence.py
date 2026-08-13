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


class ReleaseEvidenceTests(unittest.TestCase):
    def test_generates_checksums_spdx_and_license_inventory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            dist = work / "dist"
            dist.mkdir()
            (dist / "pevi-v0.1.0-windows.exe").write_bytes(b"windows")
            (dist / "pevi-v0.1.0-linux").write_bytes(b"linux")
            metadata = {
                "packages": [
                    {
                        "id": "path+file:///private/work#pevi_cli@0.1.0",
                        "name": "pevi_cli",
                        "version": "0.1.0",
                        "license": "MIT",
                        "license_file": None,
                        "source": None,
                    },
                    {
                        "id": "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0",
                        "name": "serde",
                        "version": "1.0.0",
                        "license": "MIT OR Apache-2.0",
                        "license_file": None,
                        "source": "registry+https://github.com/rust-lang/crates.io-index",
                    },
                ],
                "resolve": {
                    "nodes": [
                        {
                            "id": "path+file:///private/work#pevi_cli@0.1.0",
                            "deps": [
                                {
                                    "pkg": "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0"
                                }
                            ],
                        },
                        {
                            "id": "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0",
                            "deps": [],
                        },
                    ]
                },
                "workspace_members": ["path+file:///private/work#pevi_cli@0.1.0"],
            }
            metadata_path = work / "metadata.json"
            metadata_path.write_text(json.dumps(metadata), encoding="utf-8")

            subprocess.run(
                [
                    "python3",
                    str(GENERATOR),
                    "--dist",
                    str(dist),
                    "--metadata",
                    str(metadata_path),
                    "--repository",
                    "Tinkora/pe_version_info",
                    "--version",
                    "v0.1.0-alpha.1",
                    "--revision",
                    "a" * 40,
                    "--created",
                    "2026-08-14T00:00:00Z",
                ],
                check=True,
            )

            checksum_lines = (dist / "SHA256SUMS").read_text(encoding="utf-8").splitlines()
            self.assertEqual(
                checksum_lines,
                [
                    f"{hashlib.sha256(b'linux').hexdigest()}  pevi-v0.1.0-linux",
                    f"{hashlib.sha256(b'windows').hexdigest()}  pevi-v0.1.0-windows.exe",
                ],
            )

            sbom = json.loads((dist / "sbom.spdx.json").read_text(encoding="utf-8"))
            self.assertEqual(sbom["spdxVersion"], "SPDX-2.3")
            self.assertEqual(sbom["creationInfo"]["created"], "2026-08-14T00:00:00Z")
            self.assertEqual(len(sbom["packages"]), 2)
            self.assertIn(
                {
                    "spdxElementId": "SPDXRef-Package-pevi-cli-0.1.0",
                    "relationshipType": "DEPENDS_ON",
                    "relatedSpdxElement": "SPDXRef-Package-serde-1.0.0",
                },
                sbom["relationships"],
            )

            inventory = json.loads(
                (dist / "license_inventory.json").read_text(encoding="utf-8")
            )
            self.assertEqual(inventory["schema_version"], 1)
            self.assertNotIn("/private/work", json.dumps(inventory))
            self.assertEqual(inventory["packages"][1]["license"], "MIT OR Apache-2.0")
            notices = (dist / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")
            self.assertIn("| serde | 1.0.0 |", notices)
            self.assertNotIn("pevi_cli", notices)

    def test_spdx_ids_remain_unique_after_name_normalization(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            dist = work / "dist"
            dist.mkdir()
            (dist / "pevi").write_bytes(b"binary")
            metadata = {
                "packages": [
                    {
                        "id": "registry+https://example.invalid#index#foo-bar@1.0.0",
                        "name": "foo-bar",
                        "version": "1.0.0",
                        "license": "MIT",
                        "license_file": None,
                        "source": "registry+https://example.invalid/index",
                    },
                    {
                        "id": "registry+https://example.invalid#index#foo_bar@1.0.0",
                        "name": "foo_bar",
                        "version": "1.0.0",
                        "license": "MIT",
                        "license_file": None,
                        "source": "registry+https://example.invalid/index",
                    },
                ],
                "resolve": {"nodes": []},
                "workspace_members": [],
            }
            metadata_path = work / "metadata.json"
            metadata_path.write_text(json.dumps(metadata), encoding="utf-8")

            subprocess.run(
                [
                    "python3",
                    str(GENERATOR),
                    "--dist",
                    str(dist),
                    "--metadata",
                    str(metadata_path),
                    "--repository",
                    "Tinkora/pe_version_info",
                    "--version",
                    "v1.0.0",
                    "--revision",
                    "b" * 40,
                    "--created",
                    "2026-08-14T00:00:00Z",
                ],
                check=True,
            )

            sbom = json.loads((dist / "sbom.spdx.json").read_text(encoding="utf-8"))
            identifiers = [package["SPDXID"] for package in sbom["packages"]]
            self.assertEqual(len(identifiers), len(set(identifiers)))


if __name__ == "__main__":
    unittest.main()
