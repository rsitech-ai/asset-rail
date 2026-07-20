# AssetRail Canonical Schemas v1

This directory is the compatibility boundary between provider-specific connectors, the Rust application domain, encrypted persistence, exports, and UI projections. Provider wire payloads never become canonical schemas directly.

## Canonical envelopes

- `connector-capability.schema.json`: connector identity, read capabilities, credential schemes, and unsupported states;
- `exchange-snapshot.schema.json`: coherent account, permission, balance, network, status, quota, and freshness evidence;
- `mapping-record.schema.json`: provider identifier to canonical asset, chain, and representation mapping with state and provenance;
- `destination-profile.schema.json`: exact chain, address, memo policy, account kind, source, and verification evidence;
- `quote.schema.json`: deterministic route input, exclusions, amounts, fees, objectives, price evidence, and canonical snapshot hash;
- `audit-event.schema.json`: redacted, versioned security and operating evidence.

## Rules

1. Schema files are generated from reviewed Rust public types and checked byte-for-byte in tests.
2. Security-sensitive request types reject unknown fields.
3. Financial values serialize as decimal strings; timestamps are UTC with explicit format.
4. Canonical identifiers are validated newtypes, not provider symbols or display labels.
5. Mapping, verification, freshness, confidence, source, and unsupported states are explicit.
6. Additive compatibility is not assumed. Every changed schema receives review, tests, migration analysis, and a version decision.
7. Examples and fixtures use synthetic values only and must pass secret-pattern scans before release.
8. No schema exposes a credential, secret handle, signature, signed URL, database key, authorization header, or raw provider payload.

## Change process

Change the Rust type and behavior test first, regenerate the relevant schema, review the semantic diff, run persistence and UI compatibility tests, and commit the type and schema together. A breaking change creates a new version directory and a migration or explicit rejection path; released v1 files are never silently repurposed.
