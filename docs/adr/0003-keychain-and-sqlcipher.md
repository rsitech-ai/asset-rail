# ADR 0003: macOS Keychain and SQLCipher Storage

- Status: Accepted
- Date: 2026-07-14

## Implementation status

The connector-scoped planning credential vault and macOS Keychain adapter are implemented. SQLCipher, the separate database-key item, repositories, migrations, and recovery-state runtime are planned for Task 6 and do not exist in the current application. This ADR records the required target design; it is not evidence that encrypted structured persistence is complete.

## Context

AssetRail must retain a read-only exchange connection and useful historical evidence without placing API secrets, database keys, or sensitive account state in browser storage or plaintext files. Storage can be locked, corrupted, migrated, or restored from an older copy.

## Decision

Exchange credentials are stored as device-local generic-password items in macOS Keychain using constant, non-secret service and account attributes. Items use `WhenUnlockedThisDeviceOnly` protection. Rust wraps secret bytes in zeroizing types and exposes only opaque credential handles to application services; no command returns stored secret material.

A separate random database key is generated on first launch and stored in Keychain. Local structured state is stored in SQLCipher through `rusqlite` with foreign keys enabled, authenticated page encryption, explicit schema versions, and transactional migrations. The database may contain normalized balances, network metadata, destination profiles, mappings, quotes, reference-price observations, fee observations, sync status, and redacted audit events. It never contains API secrets, signatures, signed query strings, authorization headers, plaintext database keys, or raw onboarding payloads.

Migration failure leaves the previous schema intact. Wrong-key, locked-Keychain, and corruption states surface as distinct recovery errors. No failure triggers an automatic destructive rebuild or fixture fallback.

## Consequences

- Copying the database alone does not expose normalized account history.
- Device unlock and Keychain availability are runtime dependencies.
- Backups require an explicit encrypted export path rather than copying internal files as a supported workflow.
- Credential deletion and account-state deletion must be separately verified.

## Rejected alternatives

- Browser local storage and IndexedDB were rejected because WebView code must not own secrets or durable account state.
- Plain SQLite was rejected because account, destination, and history data are sensitive even without API secrets.
- A single Keychain item containing every secret was rejected because independent rotation and deletion are required.
- Automatic database deletion on open failure was rejected because it can destroy recoverable evidence.

## Rollback

Stop writes, preserve the encrypted database and diagnostic metadata, restore the last verified encrypted backup to a new path, and reopen with the matching Keychain key. A release rollback may open only schema versions it declares compatible; otherwise it remains read-only and directs the owner to the storage-recovery runbook.
