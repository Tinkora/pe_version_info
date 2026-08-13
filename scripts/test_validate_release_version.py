#!/usr/bin/env python3
"""Tests for release candidate version validation."""

import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = ROOT / "scripts/validate_release_version.py"


class ReleaseVersionTests(unittest.TestCase):
    def run_validator(
        self, version: str, workspace_version: str = "1.2.3"
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as temporary:
            manifest = Path(temporary) / "Cargo.toml"
            manifest.write_text(
                "[workspace]\n"
                "members = []\n\n"
                "[workspace.package]\n"
                f'version = "{workspace_version}"\n',
                encoding="utf-8",
            )
            return subprocess.run(
                ["python3", str(VALIDATOR), "--manifest", str(manifest), version],
                check=False,
                capture_output=True,
                text=True,
            )

    def test_accepts_semver_2_candidates_that_match_the_workspace(self) -> None:
        for version in (
            "0.1.0-alpha.1",
            "1.2.3",
            "1.2.3-alpha",
            "1.2.3-alpha.1+build.7",
            "1.0.0-0.3.7",
            "1.0.0-x.7.z.92",
            "1.0.0+20130313144700",
            "1.0.0-beta+exp.sha.5114f85",
        ):
            with self.subTest(version=version):
                result = self.run_validator(f"v{version}", workspace_version=version)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.strip(), f"validated v{version}")

    def test_rejects_invalid_semver_candidates(self) -> None:
        for version in (
            "1.2.3",
            "v01.2.3",
            "v1.02.3",
            "v1.2.03",
            "v1.2",
            "v1.2.3-",
            "v1.2.3+",
            "v1.2.3-alpha..1",
            "v1.2.3-01",
            "v1.2.3+build..1",
            "v1.2.3_alpha",
            "v1.2.3-١alpha",
        ):
            with self.subTest(version=version):
                result = self.run_validator(version)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("v-prefixed SemVer 2.0.0", result.stderr)

    def test_rejects_a_candidate_that_does_not_match_the_workspace(self) -> None:
        result = self.run_validator("v1.2.4")

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match workspace version v1.2.3", result.stderr)


if __name__ == "__main__":
    unittest.main()
