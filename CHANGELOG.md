# Changelog

All notable user-visible changes are recorded here. AssetRail uses semantic
versioning for source releases while the public interface remains experimental.

## [Unreleased]

No unreleased changes.

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

[Unreleased]: https://github.com/rsitech-ai/asset-rail/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/rsitech-ai/asset-rail/releases/tag/v0.1.0
