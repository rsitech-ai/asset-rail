# ADR 0005: Timestamped Reference-Price Evidence

- Status: Accepted
- Date: 2026-07-14

## Implementation status

The exact-decimal route kernel is implemented with deterministic fixture prices. Immutable live `PriceObservation` records, provider mapping, expiry, persistence, and live fiat valuation are planned and are not present in the current application.

## Context

AssetRail compares withdrawal fees and portfolio values across assets and user-selected fiat currencies. A displayed fiat value without its source, timestamp, purpose, or confidence can make a route look more precise than the evidence permits.

## Decision

Every fiat conversion consumes an immutable `PriceObservation` containing base asset, quote currency, decimal price, provider, observed-at time, received-at time, purpose, source class, confidence, and expiry. Supported initial quote currencies are EUR, USD, and PLN. The initial live provider uses Binance spot ticker data only where an approved symbol mapping exists; deterministic fixed providers remain available for tests.

The route quote stores the exact price-observation identifiers and withdrawal-fee observations used to compute it. Price purpose distinguishes portfolio display from fee valuation. Stale, missing, ambiguous, unsupported, or low-confidence prices remain visible as unavailable evidence and never silently become zero or reuse an unlabelled cached value.

No synthetic cross-rate is produced unless each leg and the cross-rate policy are versioned and captured in the quote evidence.

## Consequences

- Quotes are explainable and reproducible from retained observations.
- Some assets will have no fiat valuation until mappings and evidence are approved.
- Historical fee displays can distinguish provider changes from price changes.
- Price freshness and quote freshness are independently observable.

## Rejected alternatives

- Fetching prices directly in React was rejected because it bypasses canonical mapping, caching, and audit controls.
- Storing only the converted fiat amount was rejected because the result could not be reproduced.
- Treating all ticker symbols as mechanically derivable was rejected because asset symbols are not globally unique.
- Silent last-value fallback was rejected because stale evidence must remain explicit.

## Rollback

Disable the affected price provider or mapping and mark dependent valuations unavailable. Existing signed quote snapshots retain their original evidence and are never recalculated in place.
