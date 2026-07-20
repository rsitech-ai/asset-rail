# ADR 0002: Compile-Time Connector and Validator Registries

- Status: Accepted
- Date: 2026-07-14

## Context

The product begins with Binance but must later support more exchanges, connectors, and networks. Provider wire formats and network address rules change independently. Dynamic extensions would introduce arbitrary code and origin authority into a wallet-adjacent application.

## Decision

Exchange connectors and network validators register at compile time behind separate provider-neutral traits. A connector declares a stable identifier, read capabilities, supported credential schemes, permission inspection, synchronization behavior, rate limits, and normalization. A validator declares a canonical chain identifier, structural address rules, memo or tag policy, and evidence it can produce.

Provider wire types terminate inside the connector module. Only versioned canonical domain envelopes cross into persistence, routing, or UI projection. Adding an exchange requires contract fixtures, normalization tests, capability declarations, rate-limit tests, mapping evidence, and a threat-model update. Adding a network requires positive and negative vectors, canonical mappings, validator tests, and a threat-model update. Neither registry accepts runtime libraries, JavaScript modules, arbitrary URLs, or downloaded executable code.

## Consequences

- Exchanges and networks can evolve independently behind stable contracts.
- Every supported integration is visible in the source tree and signed application artifact.
- Adding support requires a release rather than a runtime installation.
- Unsupported capabilities are explicit states rather than silent omissions.

## Rejected alternatives

- A single provider-shaped domain model was rejected because it would leak Binance semantics through the application.
- A combined connector-and-validator interface was rejected because exchange data access and chain validation have different lifecycles.
- Dynamic plugins were rejected because extension installation would become a code-execution and supply-chain boundary.
- Arbitrary base URLs were rejected because a compromised UI could redirect signed requests and metadata.

## Rollback

Remove the registry entry for the affected connector or validator, ship a signed release, and preserve its data as unsupported historical evidence. Previously approved mappings become unavailable rather than being reassigned automatically.
