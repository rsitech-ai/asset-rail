# Releasing AssetRail

This runbook covers the v0.1.1 Developer ID-signed direct-download release. It
does not authorize Apple account changes, certificate changes, notarization
submission, App Store Connect upload, TestFlight, or App Review actions.

## Release contract

- The release source is an immutable reviewed commit on `main` with a clean
  working tree.
- GitHub Actions verifies source and secret history on pull requests and
  `main`. Local release packaging and Apple notarization remain explicit
  maintainer steps outside CI.
- The official app uses `ai.rsitech.assetrail`, version `0.1.1`, build `1`, and
  Developer ID Team `2NY8A789TN`.
- `npm run build:official` signs and verifies locally but deliberately refuses
  notarization credentials.
- Official and community bundles include the project license, NOTICE,
  copyright, trademark terms, privacy manifest, and generated Cargo/npm runtime
  dependency notices.
- Apple notarization requires a separate approval bound to the exact source SHA,
  artifact SHA-256, bundle identity, version/build, Team ID, and notary profile.
- GitHub release assets are uploaded only after notarization, stapling,
  Gatekeeper assessment, runtime proof, SBOM generation, and checksums.

## Verify the source candidate

From a fresh checkout at the proposed commit:

```bash
npm ci
npm run test:run
npm run test:config
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked --release --all-targets --all-features
npm run build:community
```

The community bundle remains an inspection artifact and is not an official
release binary.

## Build and inspect the official app

Confirm Keychain reports the exact identity, then build the clean commit:

```bash
security find-identity -v -p codesigning
npm run build:official
```

Expected output:

```text
src-tauri/target/release/bundle/macos/AssetRail.app
```

Record the source commit and submission archive digest:

```bash
git rev-parse HEAD
ditto -c -k --keepParent \
  src-tauri/target/release/bundle/macos/AssetRail.app \
  dist/AssetRail-0.1.1-notarization.zip
shasum -a 256 dist/AssetRail-0.1.1-notarization.zip
```

Stop here until the notarization approval names that exact digest, source SHA,
bundle `ai.rsitech.assetrail`, version `0.1.1` build `1`, Team `2NY8A789TN`, and
the selected Keychain notary profile.

## Notarize, staple, and package after approval

Use `xcrun notarytool submit --wait` with the approved Keychain profile. On an
accepted result, staple and validate the app, assess it with Gatekeeper, launch
the exact app, and verify its visible build/source disclosure. Then produce a
DMG using the documented Tauri DMG flow, notarize and staple that DMG if needed,
and create a ZIP from the stapled app. Do not publish an unstapled or rejected
artifact.

## SBOM, checksums, and GitHub release

Use the pinned and checksum-verified Syft version documented in
`docs/open-source/sbom/README.md`, then run:

```bash
./script/generate_sbom.sh
```

Create `SHA256SUMS` for every uploaded DMG, ZIP, and SBOM. Reject artifacts that
contain a workstation home path. Publish tag `v0.1.1` and factual release notes
from `docs/releases/v0.1.1.md`, attach only the validated assets, and verify the
downloads anonymously.

The v0.1.0 tag/release remains a superseded historical source release because
it records the license terms under which that version was published. Removing a
release cannot revoke those grants and would make the public record less clear.
