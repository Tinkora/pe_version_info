#!/usr/bin/env python3
"""Contract tests for the release workflow and consumer verification guide."""

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/release.yml"


def job_block(workflow: str, job_name: str) -> str:
    match = re.search(
        rf"(?ms)^  {re.escape(job_name)}:\n(?P<body>.*?)(?=^  [a-zA-Z0-9_]+:\n|\Z)",
        workflow,
    )
    if match is None:
        raise AssertionError(f"release workflow is missing the {job_name} job")
    return match.group(0)


class ReleaseWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")

    def test_validate_requires_a_protected_matching_tag_before_building(self) -> None:
        validate = job_block(self.workflow, "validate")
        build = job_block(self.workflow, "build")

        self.assertNotIn("refs/heads/main", validate)
        self.assertIn('refs/tags/${RELEASE_TAG}', validate)
        self.assertIn("github.ref_protected", validate)
        self.assertIn("RELEASE_REF_PROTECTED", validate)
        self.assertNotIn("run from main", validate)
        self.assertIn("needs: validate", build)

    def test_release_trigger_and_publish_job_are_present(self) -> None:
        self.assertIn("tags: ['v*.*.*']", self.workflow)
        release = job_block(self.workflow, "release")
        self.assertIn("needs: evidence", release)
        self.assertRegex(release, r"(?m)^    permissions:\n      contents: write$")
        self.assertIn("gh release create", release)
        self.assertIn("--draft", release)
        self.assertIn("-F draft=false", release)

    def test_new_release_uploads_assets_during_draft_creation(self) -> None:
        release = job_block(self.workflow, "release")

        self.assertIn('gh release create "${RELEASE_TAG}" "${assets[@]}"', release)
        self.assertNotIn("gh release upload", release)

    def test_release_retry_replaces_drafts_but_only_verifies_published_assets(self) -> None:
        release = job_block(self.workflow, "release")

        self.assertIn("--paginate --slurp", release)
        self.assertIn(".tag_name == $tag", release)
        self.assertIn(".draft == true", release)
        self.assertIn("--method DELETE", release)
        self.assertIn("Remote release asset inventory is not exact", release)
        self.assertIn("Remote release asset digest does not match", release)

    def test_privileged_evidence_job_uses_release_environment(self) -> None:
        evidence = job_block(self.workflow, "evidence")

        self.assertRegex(evidence, r"(?m)^    environment: release$")
        self.assertIn("needs: build", evidence)
        self.assertIn("attestations: write", evidence)

    def test_release_jobs_have_bounded_runtime(self) -> None:
        allowed_ranges = {
            "validate": range(1, 11),
            "build": range(10, 61),
            "evidence": range(5, 31),
            "release": range(5, 31),
        }

        for job_name, allowed in allowed_ranges.items():
            with self.subTest(job=job_name):
                block = job_block(self.workflow, job_name)
                match = re.search(r"(?m)^    timeout-minutes: ([0-9]+)$", block)
                self.assertIsNotNone(match, f"{job_name} job must have a timeout")
                self.assertIn(int(match.group(1)), allowed)

    def test_unix_artifacts_preserve_and_verify_executable_permissions(self) -> None:
        build = job_block(self.workflow, "build")

        self.assertIn(".tar.gz", build)
        self.assertRegex(build, r"tar .*-czf")
        self.assertRegex(build, r"tar .*-xzf")
        self.assertRegex(build, r'test -x "\$\{[^}]+\}[^"\n]*/pevi"')
        self.assertIn('(cd dist && shasum -a 256', build)

    def test_windows_checksum_uses_a_portable_line_ending(self) -> None:
        build = job_block(self.workflow, "build")

        self.assertIn("[System.IO.File]::WriteAllText", build)
        self.assertIn('"$checksum`n",', build)
        self.assertIn("[System.Text.Encoding]::ASCII", build)
        self.assertNotIn("Out-File -Encoding ascii", build)

    def test_attestation_guides_bind_source_ref_and_commit_digest(self) -> None:
        for relative_path in ("docs/RELEASING.md", "docs/RELEASING.zh-CN.md"):
            with self.subTest(document=relative_path):
                guide = (ROOT / relative_path).read_text(encoding="utf-8")
                self.assertIn("--source-ref", guide)
                self.assertIn("--source-digest", guide)
                self.assertIn("SHA256SUMS", guide)

    def test_all_release_actions_use_full_commit_pins(self) -> None:
        references = re.findall(
            r"^\s*uses:\s*([^@\s]+)@([^\s#]+)", self.workflow, re.MULTILINE
        )
        self.assertTrue(references)
        for action, revision in references:
            with self.subTest(action=action):
                self.assertRegex(revision, r"\A[0-9a-f]{40}\Z")


if __name__ == "__main__":
    unittest.main()
