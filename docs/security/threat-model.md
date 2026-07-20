# AssetRail Read-Only Connector Threat Model

- Scope: live read-only exchange synchronization, local destination verification, route planning, encrypted local history, and signed desktop updates
- Authority ceiling: no withdrawal, trade, transfer, bridge, staking, lending, or other money movement
- Review date: 2026-07-14

## Implementation status

This model covers the target live read-only milestone. The reviewed repository currently implements the offline route kernel, canonical connector contracts, connector-scoped Keychain vault foundation, redaction types, and bounded fixed-origin Binance transport. Permission-gated onboarding, live normalization and synchronization, SQLCipher, destination verification, live IPC and UI, exports, support bundles, and signed updates remain planned. Controls below that depend on those components are requirements, not completed evidence; see the repository readiness audit.

## Security objectives

1. A WebView, cloud, provider-response, or support-bundle compromise cannot retrieve stored credentials or acquire money-moving authority.
2. A credential with money-moving permission is rejected before it is persisted.
3. Provider and mapping ambiguity fails closed and remains visible.
4. Quotes remain reproducible from exact-decimal, timestamped evidence.
5. Stored sensitive state is encrypted and recoverable without silent destructive fallback.
6. Only signed, notarized, owner-approved releases can reach the production update channel.

## Protected assets

| Asset | Sensitivity | Required protection |
|---|---|---|
| Exchange API key and secret | Critical | Rust-only lifecycle, zeroization, device-local Keychain, never exported |
| Derived signatures and signed query strings | Critical | In-memory request scope only, never logged or persisted |
| Database encryption key | Critical | Separate device-local Keychain item, never stored with database |
| Normalized account balances and permissions | High | SQLCipher, purpose-specific projection, timestamps |
| Destination addresses, memos, labels, and verification evidence | High | SQLCipher, explicit source and verification state |
| Catalog mappings and approval records | High integrity | Versioned provenance, approval state, hash evidence |
| Price, fee, and quote evidence | High integrity | Exact decimals, immutable identifiers, source and time |
| Redacted audit events | Medium confidentiality, high integrity | Allowlisted fields, append semantics, local encryption |
| Desktop and updater public keys | High integrity | Reviewed repository configuration and signed release |
| Developer ID and updater private keys | Critical | External owner control; never present in repository or app runtime |

## Trust boundaries

| Boundary | Untrusted side | Trusted side | Enforcement |
|---|---|---|---|
| TB1 WebView to Rust IPC | React, DOM, dependencies, user input | Rust security kernel | Narrow versioned commands, input validation, no generic privileged APIs |
| TB2 Rust to macOS Keychain | Application process | OS credential service | Device-local access control, constant identifiers, opaque handles |
| TB3 Rust to SQLCipher | Filesystem and copied database | Encrypted repository | Key separation, cipher verification, transactional migrations |
| TB4 Rust to Binance | Network, DNS, provider responses | Connector normalization | Fixed HTTPS origin, TLS, signed requests, time bounds, schema validation |
| TB5 Provider data to canonical domain | Provider symbols and network names | Approved mappings | Explicit mapping state and provenance; ambiguous or unknown is ineligible |
| TB6 Destination input to route engine | Pasted or imported addresses | Validated destination profile | Chain-specific parser, memo policy, verification evidence, no inference from shape |
| TB7 Build system to installed app | Dependencies, local build tooling, distribution transport | Owner-installed signed application | Pinning, review, code signing, notarization, signed updater |
| TB8 Support/export boundary | Local sensitive state | Owner-shareable artifact | Allowlist projection, secret scanner, explicit user action |
| TB9 Future cloud boundary | Remote dashboard or service | Local planner | No credential custody, signing, secret retrieval, or money movement authority |

## Attacker capabilities

- Execute script in the WebView through dependency compromise or injection.
- Read ordinary application files and copied backups under the user account.
- Supply malformed, oversized, stale, duplicated, or semantically misleading provider responses.
- Control a destination string, memo, label, imported saved address, or clipboard contents.
- Observe logs, crash reports, exports, support bundles, and build output.
- Cause provider timeouts, rate limits, clock skew, partial responses, and permission drift.
- Compromise a dependency, update host, or build environment without possessing owner-held signing keys.
- Obtain a running unlocked Mac session; full OS administrator or kernel compromise remains outside the application's defensible boundary.

## Abuse paths and mitigations

| Abuse path | Preventive and detective controls | Result |
|---|---|---|
| WebView asks for a stored API secret | No retrieval command; no generic Keychain plugin; opaque Rust handle | Denied structurally |
| WebView attempts a withdrawal endpoint | Connector has a fixed GET-only endpoint enum; no generic request or submission command | Denied structurally |
| User supplies a withdrawal-enabled key | **Planned Task 5:** permission inspection precedes persistence; unknown or enabled money movement rejects onboarding | Connection denied once onboarding exists; current app exposes no onboarding path |
| Secret leaks through debug formatting | Secret wrappers redact `Debug` and `Display`; log fields are allowlisted; secret-pattern tests scan artifacts | Secret omitted |
| Signed query leaks through transport error | Errors map to stable redacted categories without URL, headers, body, key, or signature | Sensitive request data omitted |
| Malicious provider aliases one network as another | Provider identifiers require approved canonical mapping with version and provenance | Route remains ineligible |
| EVM-shaped address is treated as any EVM chain | Destination includes exact chain identity and asset representation; shape is only structural evidence | Cross-chain inference denied |
| Imported saved address becomes trusted | Imports enter unverified state and require explicit network, memo, and verification review | No automatic trust |
| Stale live failure falls back to fixtures | **Planned Tasks 5 and 11:** fixtures are compile/test-only; runtime cache is timestamped and visibly stale | No false-live state once live sync exists |
| Copied database exposes account history | **Planned Task 6:** SQLCipher database key is held separately in Keychain | Copy remains encrypted once persistence exists |
| Corruption triggers data loss | **Planned Task 6:** no automatic rebuild; read-only preservation and recovery runbook | Destructive fallback denied once persistence exists |
| Malicious update gains local authority | **Planned Tasks 13 and 14:** Developer ID signing, notarization, signed update metadata/artifact, staged owner release | Untrusted update rejected once signed updates exist |
| Support bundle exfiltrates address or key material | Allowlisted projections, pseudonymous IDs, no raw request/response, secret-pattern gate | Sensitive fields excluded |
| Rate limiting creates a retry storm | Endpoint token buckets, bounded jitter, `Retry-After`, circuit state, no retry on auth/schema errors | Provider load bounded |

## Residual risks

- Malware running as the same unlocked user may capture onboarding keystrokes or inspect process memory. The app minimizes secret lifetime but cannot defend against a fully compromised session.
- Binance may report incomplete or changing permission metadata. The connector fails closed, which can deny legitimate read-only users.
- Structural validation cannot prove address ownership or provider support. Verification state remains separate and visible.
- A correctly signed but malicious release remains possible if owner signing authority and review are both compromised. Key separation, staging, and reproducible evidence reduce but do not eliminate this risk.
- Market prices and provider withdrawal metadata can change between synchronization and user action. The milestone only plans and never submits, and it labels evidence freshness.
- Denial of service can prevent synchronization, Keychain access, or updates. Cached data remains labelled stale and cannot be mistaken for live data.

## Review triggers

Review this model before adding an exchange, network validator, credential mode, executable route leg, cloud command channel, new privileged Tauri capability, analytics/crash provider, update origin, or new export field. Any money-moving design requires a separate threat model and explicit owner authorization.
