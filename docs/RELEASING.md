# Releasing PE Version Info

The release workflow accepts only a protected `v`-prefixed SemVer tag whose
commit has already passed the required `main` checks. It builds the CLI on the
three supported targets, generates aggregate release evidence, verifies the
assets, and publishes a prerelease GitHub Release after the `release`
environment gate. Local success is not release authority.

## Candidate sequence

1. Build the reviewed commit with the locked Rust 1.95.0 toolchain.
2. Run `pevi inspect` on the target PE file.
3. Run `pevi plan` and review the stable JSON contract.
4. Run `pevi apply` to a separate output, then `pevi verify`.
5. Authenticode-sign only after resource edits and verification are complete.
6. Run a final independent signature check and record input/output SHA-256.
7. Create the protected tag for the exact workspace version, for example
   `v0.1.0-alpha.2`, and wait for the tag-triggered `Release` workflow.
8. Download the three target archives and aggregate evidence artifact. Verify
   `SHA256SUMS`, `sbom.spdx.json`, `license_inventory.json`, and
   `THIRD_PARTY_NOTICES.md`. The SBOM and license inventory contain the union
   of normal and build dependencies reachable from `pevi_cli` for the three
   released targets; development-only dependencies are excluded. The notice
   file embeds the license and notice files from each selected third-party
   Cargo package, and evidence generation fails when a declared license cannot
   be mapped unambiguously to the package's files.
9. Verify both the build-provenance and SBOM attestations with GitHub CLI for
   each candidate binary.
10. The workflow creates and publishes the prerelease only after repository
    rules and the protected `release` environment are satisfied.

For each downloaded binary, verify both predicates and bind the signer to this
repository's release workflow:

```bash
gh attestation verify PATH_TO_BINARY \
  --repo Tinkora/pe_version_info \
  --signer-workflow Tinkora/pe_version_info/.github/workflows/release.yml \
  --source-ref refs/tags/v0.1.0-alpha.2 \
  --source-digest COMMIT_SHA
gh attestation verify PATH_TO_BINARY \
  --repo Tinkora/pe_version_info \
  --signer-workflow Tinkora/pe_version_info/.github/workflows/release.yml \
  --predicate-type https://spdx.dev/Document/v2.3 \
  --source-ref refs/tags/v0.1.0-alpha.2 \
  --source-digest COMMIT_SHA
```

Replace `COMMIT_SHA` with the exact reviewed commit used by the candidate and
keep the source ref and digest identical for both checks.

Never edit signed release binaries in place. Keep the prior artifact and its
checksum for rollback, and never move an immutable tag to hide a bad artifact.
