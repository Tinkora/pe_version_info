# Releasing PE Version Info

This repository remains Draft until the exact pushed commit has passing hosted
native, documentation, and supply-chain checks. Local success is not release
authority. The release workflow validates a `v`-prefixed SemVer value against
the workspace version, builds candidate artifacts, and generates aggregate
release evidence. It does not create tags or GitHub Releases automatically.

## Candidate sequence

1. Build the reviewed commit with the locked Rust 1.95.0 toolchain.
2. Run `pevi inspect` on the target PE file.
3. Run `pevi plan` and review the stable JSON contract.
4. Run `pevi apply` to a separate output, then `pevi verify`.
5. Authenticode-sign only after resource edits and verification are complete.
6. Run a final independent signature check and record input/output SHA-256.
7. Dispatch `Release candidate build` with the exact workspace version, for
   example `v0.1.0-alpha.1`.
8. Download the three target binaries and aggregate evidence artifact. Verify
   `SHA256SUMS`, `sbom.spdx.json`, `license_inventory.json`, and
   `THIRD_PARTY_NOTICES.md`. The SBOM and license inventory contain the union
   of normal and build dependencies reachable from `pevi_cli` for the three
   released targets; development-only dependencies are excluded. The notice
   file embeds the license and notice files from each selected third-party
   Cargo package, and evidence generation fails when a declared license cannot
   be mapped unambiguously to the package's files.
9. Verify both the build-provenance and SBOM attestations with GitHub CLI for
   each candidate binary.
10. Publish a prerelease only after repository rules, protected environments, and
   hosted checks are verified by an authorized maintainer.

For each downloaded binary, verify both predicates and bind the signer to this
repository's release workflow:

```bash
gh attestation verify PATH_TO_BINARY \
  --repo Tinkora/pe_version_info \
  --signer-workflow Tinkora/pe_version_info/.github/workflows/release.yml \
  --source-ref refs/tags/v0.1.0-alpha.1 \
  --source-digest COMMIT_SHA
gh attestation verify PATH_TO_BINARY \
  --repo Tinkora/pe_version_info \
  --signer-workflow Tinkora/pe_version_info/.github/workflows/release.yml \
  --predicate-type https://spdx.dev/Document/v2.3 \
  --source-ref refs/tags/v0.1.0-alpha.1 \
  --source-digest COMMIT_SHA
```

Replace `COMMIT_SHA` with the exact reviewed commit used by the candidate and
keep the source ref and digest identical for both checks.

Never edit signed release binaries in place. Keep the prior artifact and its
checksum for rollback, and never move an immutable tag to hide a bad artifact.
