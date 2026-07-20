# Authorization Matrix

This matrix defines the maximum target authority for the live read-only milestone. The current fixture application exposes a strict subset: planner IPC only. Credential onboarding, live connector orchestration, SQLCipher repositories, export, and updater services are not yet wired into the application.

`ALLOW` means the component may perform the operation through the named narrow interface. `DENY` means no interface or credential for the operation may exist in this milestone.

| Component | Read UI projection | Submit onboarding secret | Retrieve secret | Provider GET allowlist | Arbitrary network | Encrypted state | Export allowlist | Money movement |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| React WebView | ALLOW | ALLOW, one command input | **DENY** | **DENY** | **DENY** | **DENY** | ALLOW, request only | **DENY** |
| Tauri IPC adapter | ALLOW | ALLOW, immediate handoff | **DENY** | **DENY** | **DENY** | **DENY** | ALLOW, typed request | **DENY** |
| Rust application service | ALLOW | ALLOW, planning mode only | **DENY**, handle use only | ALLOW through connector | **DENY** | ALLOW through repository | ALLOW through projector | **DENY** |
| Credential vault | **DENY** | ALLOW, validated planning credential | ALLOW to connector closure only | **DENY** | **DENY** | **DENY** | **DENY** | **DENY** |
| Binance connector | **DENY** | ALLOW, opaque handle | **DENY**, scoped signing only | ALLOW, fixed GET endpoints | **DENY** | **DENY** | **DENY** | **DENY** |
| SQLCipher repository | **DENY** | **DENY** | **DENY** | **DENY** | **DENY** | ALLOW, canonical records | **DENY** | **DENY** |
| Export projector | ALLOW, redacted fields | **DENY** | **DENY** | **DENY** | **DENY** | Read allowlisted records | ALLOW | **DENY** |
| Future cloud service | ALLOW, separately approved summaries | **DENY** | **DENY** | **DENY** | **DENY** | **DENY** | **DENY** by default | **DENY** |
| Local build and release tooling | **DENY** for real account data | **DENY** | **DENY** | **DENY** | Fixed dependency origins only | **DENY** | Build artifacts only | **DENY** |

## Tauri capability rules

- The main window receives no built-in core or plugin permissions. Its capability selects an empty permission list explicitly; `core:default` is forbidden because it expands to unused image, path, menu, tray, WebView, window, event, resource, and application commands.
- Application-owned custom commands are purpose-specific and separately registered by the Rust host. Before additional windows or onboarding commands are added, the Tauri application manifest must constrain which windows may invoke them.
- Generic shell, HTTP, filesystem, SQL, Keychain, Stronghold, clipboard-read, process, and opener permissions are absent unless a later reviewed feature supplies a narrower contract.
- Navigation and content-security policy allow only packaged application content and explicitly required local IPC behavior.
- A command that accepts a URL, SQL string, filesystem path outside an application-owned picker result, shell command, provider endpoint, or credential identifier is rejected during review.

## Credential mode rules

| Mode | Constructible now | Persistable now | Provider reads | Withdrawal | Trade | Transfer |
|---|---:|---:|---:|---:|---:|---:|
| `PLANNING` | ALLOW after permission inspection | ALLOW | Fixed allowlist | DENY | DENY | DENY |
| `EXECUTION` | DENY | DENY | DENY | DENY | DENY | DENY |
| `TRADING` | DENY | DENY | DENY | DENY | DENY | DENY |

## Change control

Changing any `DENY` to `ALLOW` requires an ADR, threat-model update, test proving the old path remains unavailable where applicable, security review, and explicit owner approval. Configuration or cloud flags cannot alter this matrix.
