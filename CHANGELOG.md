# Changelog

All notable user-visible changes are recorded here. AssetRail uses semantic
versioning for source releases while the public interface remains experimental.

## [Unreleased]

No unreleased changes.

## [0.1.2] - 2026-08-13

### Changed

- Updated the pinned frontend and Rust dependency set to current compatible releases.
- Refreshed transitive packages to resolve the npm advisories present in 0.1.1.
- Preserved AssetRail's offline, read-only planning and non-execution safety boundary.

## [0.1.1] - 2026-07-21

### Changed

- Adopted Apache-2.0 for original source code and project documentation in
  v0.1.1 and later.
- Recorded Rafal Sikora as copyright owner and RSI Tech as public maintainer,
  with `https://rsitech.ai` and `info@rsitech.ai` as the canonical contacts.
- Updated package, Cargo, Tauri, community-build, and in-app release metadata to
  version 0.1.1 and Apache-2.0.
- Added a fail-closed official build path for the Developer ID direct-download
  route. It requires an exact clean source commit and the expected Team
  `2NY8A789TN` identity, while keeping notarization credentials out of the local
  build step.
- Added clean-commit-bound Cargo and npm runtime dependency notices, with the
  project license, NOTICE, copyright, and trademark terms embedded in release
  bundles.
- Removed assistant workspace material from the public tree and ignored local
  IDE, agent, and monetization working notes.
- Restored a minimal GitHub Actions verification and secret-scan workflow.
- Historical `v0.1.0` license grants remain unchanged. Its release and tag are
  retained as the accurate record of the first public source release.

### Limitations

- Official downloadable Apple binaries are published only after the exact bundle
  is notarized, stapled, Gatekeeper-validated, and runtime-proven.
- Source archives, SBOMs, and checksums may ship before notarization completes.

## [0.1.0] - 2026-07-20

### Added

- Offline macOS planner with deterministic synthetic exchange and destination
  fixtures.
- Exact-decimal Rust route evaluation with reason-coded exclusions.
- Strict browser-to-Rust snapshot validation and explicit chain/asset identity.
- Empty Tauri built-in permission set for the main window.
- Distinct, source-bound community build with path-leakage checks.
- MPL-2.0 source license, CC-BY-4.0 documentation license, DCO contribution
  policy, private security reporting, governance, and community templates.

### Limitations

- No live exchange connection, wallet connection, trade, transfer, withdrawal,
  credential entry, telemetry, remote service, or persistent user data.
- Apple silicon is the only runtime architecture verified for this release.
- No Developer ID-signed or notarized binary is published.
- CI is intentionally not part of the release process.

[Unreleased]: https://github.com/rsitech-ai/asset-rail/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/rsitech-ai/asset-rail/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/rsitech-ai/asset-rail/releases/tag/v0.1.0
