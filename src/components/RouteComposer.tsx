import { Icon } from "./Icon";
import { withdrawalAmountError } from "../planner/decimal";
import type { DestinationView, PlannerRequest, RecommendationObjective } from "../planner/types";

const objectives: Array<{ value: RecommendationObjective; label: string }> = [
  { value: "SAFEST", label: "Safest" },
  { value: "CHEAPEST", label: "Cheapest" },
  { value: "FASTEST", label: "Fastest" },
  { value: "BALANCED", label: "Balanced" },
  { value: "NATIVE_ONLY", label: "Native only" },
];

interface Props {
  request: PlannerRequest;
  destinations: DestinationView[];
  busy: boolean;
  onChange: (next: PlannerRequest) => void;
  onObjective: (objective: RecommendationObjective) => void;
  onCompare: () => void;
}

export function RouteComposer({ request, destinations, busy, onChange, onObjective, onCompare }: Props) {
  const destination = destinations.find((item) => item.id === request.destinationId);
  const amountError = withdrawalAmountError(request.amount);
  return (
    <section className="composer" aria-labelledby="composer-heading">
      <div className="section-heading composer-heading">
        <div><span className="eyebrow">Route request</span><h2 id="composer-heading">Compose a direct withdrawal</h2></div>
        <span className="read-only-badge"><Icon name="lock" size={13} /> Read-only</span>
      </div>

      <div className="endpoint-grid">
        <div className="endpoint source-endpoint">
          <span className="endpoint-kicker">From</span>
          <div className="endpoint-title"><span className="binance-mark">B</span><div><strong>Binance Spot</strong><small>Planning account · local fixture</small></div></div>
          <div className="endpoint-meta"><span>Asset</span><strong>{request.assetSymbol}</strong></div>
        </div>
        <div className="route-connector" aria-hidden="true"><span className="connector-pulse" /><Icon name="arrow" size={20} /></div>
        <div className="endpoint destination-endpoint">
          <label className="endpoint-kicker" htmlFor="destination">To fixture destination</label>
          <select id="destination" value={request.destinationId} onChange={(event) => onChange({ ...request, destinationId: event.target.value })}>
            {destinations.map((item) => <option value={item.id} key={item.id}>{item.name}</option>)}
          </select>
          {destination && <div className="endpoint-meta"><span>{destination.chainId}</span><strong>{destination.addressLabel}</strong></div>}
        </div>
      </div>

      <div className="request-controls">
        <label className="amount-field"><span>Withdrawal amount</span><span className={`amount-input ${amountError ? "is-invalid" : ""}`}><input aria-label="Withdrawal amount" value={request.amount} inputMode="decimal" onChange={(event) => onChange({ ...request, amount: event.target.value })} aria-describedby="amount-help" aria-invalid={amountError ? "true" : "false"}/><strong>{request.assetSymbol}</strong></span><small id="amount-help" className={amountError ? "input-error" : ""}>{amountError ?? "Exact decimal · fees deducted from amount"}</small></label>
        <fieldset className="objective-field"><legend>Recommendation objective</legend><div className="objective-switcher">
          {objectives.map((objective) => <button type="button" key={objective.value} className={request.objective === objective.value ? "is-active" : ""} aria-pressed={request.objective === objective.value} onClick={() => onObjective(objective.value)} disabled={amountError !== null}>{objective.label}</button>)}
        </div></fieldset>
        <button className="primary-action" onClick={onCompare} disabled={busy || amountError !== null}>{busy ? "Evaluating…" : "Compare routes"}<Icon name="arrow" size={17}/></button>
      </div>
    </section>
  );
}
