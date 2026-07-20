# ADR 0001: Rust-Owned Security Kernel

- Status: Accepted
- Date: 2026-07-14

## Context

AssetRail is a Tauri desktop application that will read real exchange accounts and real wallet destinations. Its React WebView is useful for presentation but is exposed to browser-class injection and dependency risks. The initial milestone must be incapable of moving money, while later exchanges and networks must be addable without weakening the boundary.

## Decision

Rust owns every privileged and financially significant operation: credential lifecycle, provider transport, request signing, permission inspection, encrypted persistence, canonical normalization, mapping governance, destination validation, exact-decimal route evaluation, audit creation, and export projection.

The WebView receives only narrow, versioned, purpose-specific Tauri commands. It cannot access generic HTTP, shell, filesystem, SQL, Keychain, updater-signing, or secret-retrieval functions. The Rust command allowlist contains no withdrawal, trade, transfer, bridge, or arbitrary provider request command. Financial amounts remain decimal strings at serialization boundaries and `rust_decimal::Decimal` in domain logic.

The kernel fails closed on unsupported providers, networks, asset representations, permission states, mappings, stale evidence, or schema versions. Unsafe Rust is prohibited in application code.

## Consequences

- Frontend compromise cannot directly retrieve stored credentials or invoke money movement.
- Provider and validator integrations require Rust code, review, fixtures, and tests.
- More domain logic must be projected into UI-specific response types.
- The read-only milestone remains structurally separate from any future execution milestone.

## Rejected alternatives

- JavaScript provider SDKs were rejected because they expose secrets and signing material to the WebView runtime.
- Generic Tauri HTTP, filesystem, SQL, or Keychain plugins were rejected because their authority is wider than the product use case.
- A cloud credential vault was rejected because a cloud compromise would become an account-wide credential compromise.
- Floating-point financial logic was rejected because route ordering and displayed net values must be deterministic and exact.

## Rollback

Disable the affected narrow command, revert to the last known-good signed desktop build, and retain encrypted data for read-only recovery. Do not replace a failed live source with fixture data. Rolling back this ADR requires a new security review and explicit owner approval because it changes the primary trust boundary.
