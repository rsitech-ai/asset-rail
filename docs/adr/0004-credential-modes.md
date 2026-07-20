# ADR 0004: Typed Credential Modes

- Status: Accepted
- Date: 2026-07-14

## Implementation status

Mode-typed, connector-scoped credential handles and sealed vault operations are implemented. Remote Binance permission inspection and the onboarding application service are not implemented. The current application exposes no credential-onboarding IPC, and real credentials must not be entered or persisted. Task 5 must inspect a transient credential and reject unsafe or ambiguous authority before the application service may call the vault.

## Context

The production specification anticipates planning, execution, and trading credentials in different milestones. A string flag or shared credential handle would make accidental authority reuse possible as the codebase grows.

## Decision

Credential mode is both a serialized enum and a Rust type parameter. `CredentialHandle<Planning>`, `CredentialHandle<Execution>`, and `CredentialHandle<Trading>` are not interchangeable. Handles also carry a validated connector identifier and credential scheme so one connector cannot authorize with another connector's handle. Constructors are private to their security modules.

The target live milestone can construct, persist, load, and use only planning credentials. Before persistence, a planning credential must permit required read operations and must fail connection if the inspected posture enables withdrawal, spot or margin trading, futures, options, portfolio margin, FIX trading, internal transfer, universal transfer, or another money-moving capability. Ambiguous, unknown, or unavailable permission evidence also fails closed. IP restrictions and least-privilege posture are displayed as separate evidence and cannot weaken the hard mode check.

No execution or trading service, command, endpoint, vault namespace, or conversion function is compiled into this milestone.

## Consequences

- Later execution work cannot silently reuse planning credentials.
- Some provider keys that appear read-only to an operator may be rejected when the provider cannot prove the required posture.
- New providers must map their permission models into the canonical posture with explicit unknown states.
- Permission drift requires re-inspection and can disable synchronization.

## Rejected alternatives

- One credential type with a runtime string was rejected because it makes authority mistakes easy to compile.
- Accepting a key and showing a warning was rejected because warnings do not enforce least privilege.
- Inferring permissions from successful read calls was rejected because successful reads do not prove money movement is disabled.
- Reusing one credential for future execution was rejected because permission separation is an intentional product control.

## Rollback

Disconnect and delete the affected Keychain item, revoke the provider key, and retain only redacted historical records. Relaxing a credential posture requires a separately approved design and release; it is not a configuration rollback.
