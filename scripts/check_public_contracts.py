#!/usr/bin/env python3
"""Check repository-owned public contract files without network access."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def main() -> None:
    manifest = json.loads((ROOT / "plugin/pe-version-info/.codex-plugin/plugin.json").read_text())
    assert manifest["name"] == "pe-version-info"
    assert (ROOT / "plugin/pe-version-info/skills/pe-version-info/SKILL.md").is_file()
    marketplace = json.loads((ROOT / ".agents/plugins/marketplace.json").read_text())
    entry = next(item for item in marketplace["plugins"] if item["name"] == "pe-version-info")
    source_path = ROOT / entry["source"]["path"]
    assert source_path.is_dir(), f"marketplace source path does not exist: {source_path}"
    assert entry["policy"]["installation"] == "AVAILABLE"
    assert entry["policy"]["authentication"] == "ON_INSTALL"
    assert entry["category"] == "Productivity"
    for schema_name in ("pevi_config_v1.json", "pevi_report_v1.json"):
        schema = json.loads((ROOT / "schemas" / schema_name).read_text())
        assert schema["properties"]["schema_version"]["const"] == 1
    windows_evidence = ROOT / "scripts/test_windows_evidence.ps1"
    assert windows_evidence.is_file(), "Windows evidence script is required"
    evidence_text = windows_evidence.read_text(encoding="UTF-8")
    for marker in (
        "Get-AuthenticodeSignature",
        "Set-AuthenticodeSignature",
        "GetVersionInfo",
        "ExtractIconEx",
        "--allow-signed-input",
        "--acknowledge-signature-invalidation",
        "cdylib",
    ):
        assert marker in evidence_text, f"Windows evidence is missing {marker}"
    quality_workflow = (ROOT / ".github/workflows/quality.yml").read_text(encoding="UTF-8")
    assert "scripts/test_windows_evidence.ps1" in quality_workflow
    assert "windows-native-evidence" in quality_workflow
    assert "scripts/test_clean_consumer.py" in quality_workflow
    assert "clean-consumer-${{ runner.os }}" in quality_workflow
    windows_evidence = (ROOT / "scripts/test_windows_evidence.ps1").read_text(
        encoding="UTF-8"
    )
    assert "CertificateRequest" in windows_evidence
    assert "New-SelfSignedCertificate" not in windows_evidence
    assert "StoreName]::TrustedPublisher" in windows_evidence
    assert "StoreName]::My" in windows_evidence
    assert "Assert-CodeSigningCertificateChain" in windows_evidence
    assert "X509ChainTrustMode]::CustomRootTrust" in windows_evidence
    assert "X509KeyStorageFlags]::PersistKeySet" in windows_evidence
    assert "X509KeyStorageFlags]::UserKeySet" in windows_evidence
    assert "Windows evidence phase: trust signing certificate" in windows_evidence
    release_workflow = (ROOT / ".github/workflows/release.yml").read_text(encoding="UTF-8")
    for marker in (
        "scripts/generate_release_evidence.py",
        "scripts/validate_release_version.py",
        "SHA256SUMS",
        "sbom.spdx.json",
        "license_inventory.json",
        "THIRD_PARTY_NOTICES.md",
        "actions/attest@",
        "subject-checksums",
        "sbom-path",
    ):
        assert marker in release_workflow, f"release workflow is missing {marker}"
    print("public contract checks passed")

if __name__ == "__main__":
    main()
