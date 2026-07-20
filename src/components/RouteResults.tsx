import { useState } from "react";
import { Icon } from "./Icon";
import type { DestinationView, ExcludedRouteView, RouteView } from "../planner/types";

interface Props { routes: RouteView[]; excluded: ExcludedRouteView[]; asset: string; destination?: DestinationView; }

function titleCase(value: string) { return value.toLowerCase().replaceAll("_", " ").replace(/^./, (letter) => letter.toUpperCase()); }

export function RouteResults({ routes, excluded, asset, destination }: Props) {
  const [showExcluded, setShowExcluded] = useState(false);
  return (
    <section className="results" aria-labelledby="results-heading">
      <div className="section-heading results-heading">
        <div><span className="eyebrow">Hard filters passed first</span><h2 id="results-heading">Eligible routes</h2></div>
        <span className="eligible-count"><Icon name="check" size={14}/>{routes.length} eligible</span>
      </div>
      {routes.length === 0 ? (
        <div className="no-route"><Icon name="shield" size={28}/><div><h3>No route can be recommended</h3><p>Review the exclusions below. AssetRail will not substitute a network or token representation.</p></div></div>
      ) : routes.map((route) => (
        <article className="route-card" key={route.id}>
          <div className="route-card-head"><div>{route.recommended && <span className="recommended-label"><span/>Recommended</span>}<h3>{route.networkName}</h3><p>{route.explanation}</p></div><div className="route-status"><span className={`risk risk-${route.riskTier.toLowerCase()}`}>{titleCase(route.riskTier)} risk</span><strong>{route.exchangeCode}</strong></div></div>
          <div className="visual-rail">
            <div className="rail-node"><span className="node-mark">{asset.slice(0,1)}</span><div><small>Source asset</small><strong>{asset}</strong></div></div>
            <div className="rail-line"><span/><i/><Icon name="arrow" size={18}/></div>
            <div className="rail-node network-node"><span className="node-mark"><Icon name="route" size={18}/></span><div><small>Canonical network</small><strong>{route.chainId}</strong></div></div>
            <div className="rail-line"><span/><i/><Icon name="arrow" size={18}/></div>
            <div className="rail-node"><span className="node-mark"><Icon name="wallet" size={18}/></span><div><small>Fixture destination</small><strong>{destination?.name ?? "Destination"}</strong></div></div>
          </div>
          <div className="representation"><span>Exact asset representation</span><code>{route.representationId}</code></div>
          <dl className="route-metrics">
            <div><dt>Gross amount</dt><dd>{route.grossAmount} {asset}</dd></div>
            <div><dt>Network fee</dt><dd>{route.fee} {asset}<small>{route.effectiveFeePercent}% effective</small></dd></div>
            <div className="net-metric"><dt>Expected net</dt><dd>{route.netReceived} {asset}</dd></div>
            <div><dt>Exchange estimate</dt><dd>≈ {route.estimatedMinutes} min<small>Not an arrival guarantee</small></dd></div>
          </dl>
        </article>
      ))}
      <div className={`exclusions ${showExcluded ? "is-open" : ""}`}>
        <button className="exclusion-toggle" onClick={() => setShowExcluded((current) => !current)} aria-expanded={showExcluded} aria-label={`Review ${excluded.length} excluded routes`}>
          <span><span className="exclusion-icon">!</span><strong>Review {excluded.length} excluded routes</strong><small>Safety evidence remains visible</small></span><Icon name="chevron" size={18}/>
        </button>
        {showExcluded && <div className="exclusion-list">{excluded.map((route) => (
          <article key={`${route.exchangeCode}-${route.code}`}><div className="excluded-network"><strong>{route.exchangeCode}</strong><span>{route.networkName}</span></div><div><h3>{route.title}</h3><p>{route.detail}</p><code>{route.code}</code></div></article>
        ))}</div>}
      </div>
    </section>
  );
}
