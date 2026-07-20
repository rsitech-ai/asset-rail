# Redaction Contract

## Rule

Sensitive values are excluded by construction, not cleaned after serialization. Logs, errors, audit events, crash context, analytics, exports, support bundles, fixtures, snapshots, and build output use explicit allowlist projections. Raw provider requests and responses are never diagnostic payloads.

## Never emit

- API keys, API secrets, private signing keys, database keys, Keychain values, or opaque secret handles;
- signatures, signed query strings, `X-MBX-APIKEY`, authorization headers, cookies, or raw request URLs containing query parameters;
- onboarding command payloads, raw provider bodies, SQLCipher pragmas containing key material, or environment dumps;
- full destination addresses, memos, tags, account numbers, user labels, device identifiers, or absolute home-directory paths;
- exact balance lists in crash reports, analytics, or support bundles;
- Apple signing credentials, updater private keys, notarization credentials, or build-environment secret names paired with values.

## Allowed structured fields

- stable event name and schema version;
- connector identifier, endpoint class, capability, and result category;
- pseudonymous local account identifier generated independently of provider identifiers;
- HTTP status class and provider error code only after an explicit mapping review;
- duration, retry count, token-bucket state, clock-offset magnitude, record counts, freshness age, and correlation identifier;
- asset, chain, and mapping identifiers only in owner-requested local exports;
- truncated address fingerprint produced by a domain-separated cryptographic hash, never address prefix or suffix;
- application version, operating-system version, schema version, and migration identifier.

## Error mapping

Boundary errors are mapped to stable categories: authentication, unsafe permission posture, clock, rate limit, provider unavailable, provider schema, mapping, validation, storage locked, storage corruption, migration, update verification, and internal redacted. Human messages describe the next safe action and do not interpolate raw error objects.

## Logging API contract

- Secret wrapper types implement redacted `Debug` and `Display` or implement neither.
- Network helpers accept a structured event sink rather than arbitrary formatted request data.
- Audit constructors accept typed allowlisted fields; they do not accept generic JSON maps.
- Production builds disable tracing of request/response bodies and signed query strings.
- Panic hooks and crash reporters receive only the stable category and correlation identifier.

## Verification

Current foundation coverage injects synthetic credential, signature, header, and address markers into the implemented redaction wrappers and diagnostic event, then proves their rendered forms do not contain those markers. Repository configuration tests also enforce the presence of the release secret-scanning gate.

Release checks must run Gitleaks across repository history and scan implemented logs, errors, snapshots, exports, support bundles, and build artifacts with representative canaries. A match blocks release. Surfaces that do not exist yet are requirements, not completed evidence. Review also rejects `Debug` derivation on types that transitively contain secret material.

## Incident response

If a forbidden value is emitted, stop distribution, disable the affected diagnostic or export path, preserve access-controlled evidence, revoke exposed provider or signing credentials, rotate database material when implicated, and ship a reviewed signed fix before restoring the path.
