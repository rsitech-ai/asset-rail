# Releasing AssetRail

This runbook covers the source-only `0.1.x` GitHub release. It does not authorize
package-registry publication, App Store submission, Developer ID signing, or
notarization.

## Release contract

- The release commit is immutable and has a clean working tree.
- CI is not used as a gate. Every required command runs locally.
- The community app is an inspection artifact, not a published binary.
- The GitHub release contains source archives, SBOMs, and checksums only.
- The README and in-app source disclosure identify the same repository,
  revision, version, license, and support boundary.

## Prepare the candidate

From a fresh worktree at the proposed commit:

```bash
npm ci
npm run test:run
npm run test:config
npm run build
npm run verify:security
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked --release --all-targets --all-features
npm audit --audit-level=high
cargo audit --file src-tauri/Cargo.lock
npm run build:community
```

Verify the community bundle's strict ad-hoc signature, exact source disclosure,
distinct identity, fixture planner, and absence of workstation paths. Launch
the exact bundle that was built.

Run final secret scans:

```bash
gitleaks dir . --no-banner --redact
gitleaks git . --no-banner --redact
```

## SBOM and checksums

Use the pinned and checksum-verified Syft version documented in
`docs/open-source/sbom/README.md`, then run:

```bash
./script/generate_sbom.sh
```

Regenerate the source and npm SBOMs after any release-commit change. Record
SHA-256 checksums for every uploaded SBOM and source archive. Reject artifacts
that contain a workstation home path.

## Publish

1. Confirm the default branch matches the reviewed commit.
2. Create annotated tag `v0.1.0` at that commit.
3. Publish factual release notes based on `CHANGELOG.md`.
4. Attach the SBOMs and checksum manifest; do not attach the ad-hoc-signed app.
5. Inspect the repository and release while signed out.
6. Clone anonymously, follow the README, rebuild, and smoke the community app.
7. Re-read branch protection, security settings, and private vulnerability
   reporting after the visibility change.

If the public source or release contains sensitive material, make the repository
private when possible, revoke affected credentials, preserve incident evidence,
and coordinate remediation through `SECURITY.md`. Do not rewrite public history
or delete a release without a separate incident decision.
