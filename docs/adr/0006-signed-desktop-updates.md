# ADR 0006: Signed and Staged Desktop Updates

- Status: Accepted
- Date: 2026-07-14

## Implementation status

This is the required release design, not current distribution evidence. The repository currently produces only a locally ad-hoc-signed debug bundle. Developer ID signing, hardened-runtime verification, notarization, stapling, updater configuration, update signing, staged publication, and rollback artifact proof remain uncompleted owner/release gates.

## Context

A malicious desktop update could bypass every local security boundary. Distribution must distinguish repository readiness from signing, notarization, update publication, and owner-controlled rollout.

## Decision

Release builds use hardened runtime, Developer ID Application signing, Apple notarization, and stapling. Update metadata and artifacts are signed with a dedicated Tauri updater key whose private material never enters the repository, developer shell history, application bundle, logs, or build output. The application embeds only the public verification key and a fixed HTTPS update origin.

Releases are immutable, versioned, checksummed, and staged. Publication requires the automated security matrix, installed-app smoke evidence, signature verification, notarization assessment, update-signature verification, and an explicit owner release decision. The previous known-good signed artifact and metadata remain available for rollback. Updates never weaken the local credential-mode or endpoint allowlist through a server flag.

## Consequences

- Repository completion alone cannot be labelled signed, notarized, or public-production-ready.
- Apple and updater signing credentials remain external owner-controlled gates.
- Emergency response can pause the update feed without changing installed applications.
- Rollback is a new signed release or a documented restore of the previous signed artifact, not an unsigned downgrade.

## Rejected alternatives

- Unsigned in-app updates were rejected because transport security alone does not establish publisher authenticity.
- A private update key in build configuration or the repository was rejected because compromise would authorize malicious releases.
- Automatic rollout to every device was rejected because wallet-adjacent changes need staged observation.
- Cloud flags that relax local security rules were rejected because server compromise must not expand desktop authority.

## Rollback

Pause the update manifest, preserve evidence, notify affected owners through the approved channel, and publish a higher-version signed build based on the last known-good source and compatible data schema. Revoke and rotate the updater key if signing material may be compromised.
