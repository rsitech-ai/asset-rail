import type { BalanceView } from "../planner/types";

interface Props {
  balances: BalanceView[];
  selected: string;
  onSelect: (asset: string) => void;
}

function assetMark(asset: string) {
  return asset === "USDC" ? "$" : asset.slice(0, 1);
}

export function SourceRail({ balances, selected, onSelect }: Props) {
  return (
    <aside className="source-rail" aria-label="Available balances">
      <div className="section-heading">
        <div><span className="eyebrow">Source inventory</span><h2>Balances</h2></div>
        <span className="count">{balances.length}</span>
      </div>
      <div className="balance-list">
        {balances.map((balance) => (
          <button
            className={`balance-row ${selected === balance.assetSymbol ? "is-selected" : ""}`}
            key={balance.assetSymbol}
            onClick={() => onSelect(balance.assetSymbol)}
            aria-pressed={selected === balance.assetSymbol}
            aria-label={`Select ${balance.assetSymbol} balance`}
          >
            <span className={`asset-mark asset-${balance.assetSymbol.toLowerCase()}`}>{assetMark(balance.assetSymbol)}</span>
            <span className="balance-identity"><strong>{balance.assetSymbol}</strong><small>{balance.assetName}</small></span>
            <span className="balance-values"><strong>{balance.available}</strong><small>{balance.fiatValue} {balance.fiatCurrency}</small></span>
          </button>
        ))}
      </div>
      <div className="rail-note">
        <span className="note-dot" />
        <p><strong>Non-zero only</strong><br />Locked balances remain separate.</p>
      </div>
    </aside>
  );
}
