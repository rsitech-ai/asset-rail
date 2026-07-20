# Security policy

## Supported versions

Security fixes target the latest published `0.1.x` source release and the
default branch. Older commits and locally modified builds are not maintained as
separate security-support lines.

## Report a vulnerability privately

Do not open a public issue for a suspected vulnerability.

Use [GitHub Private Vulnerability Reporting](https://github.com/rsitech-ai/asset-rail/security/advisories/new)
as the primary channel. If that form is unavailable, email
[info@rsitech.ai](mailto:info@rsitech.ai) with the subject
`AssetRail security report`.

Include:

- the affected version or commit;
- the affected component and platform;
- reproduction steps using synthetic data;
- expected and observed impact;
- a minimal proof of concept when safe;
- whether you believe disclosure is time-sensitive.

Do not send real exchange credentials, wallet private keys, seed phrases,
session tokens, personal financial data, or private wallet addresses. Redact
logs to the minimum detail required to reproduce the issue.

The maintainer aims to acknowledge a complete report within three business
days. Acknowledgement is not a promise of a particular fix or disclosure date.
The project does not currently operate a bug-bounty program.

## Scope

High-priority reports include failures in IPC validation, route eligibility,
exact-decimal handling, redaction, synthetic-fixture enforcement, Tauri
capability isolation, community-build source disclosure, or credential-vault
boundaries.

The published prototype does not expose a hosted service, exchange login,
credential entry, live synchronization, trade, transfer, or withdrawal API.
Reports that assume those unavailable surfaces should identify the downstream
fork or unpublished component they concern.

## Coordinated disclosure

Please allow the maintainer time to validate and remediate a report before
public disclosure. The maintainer will use a GitHub Security Advisory when
private collaboration and attribution are appropriate.
