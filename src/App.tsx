import { useEffect, useRef, useState } from "react";
import { BuildInformation } from "./components/BuildInformation";
import { EvidenceRail } from "./components/EvidenceRail";
import { Icon } from "./components/Icon";
import { RouteComposer } from "./components/RouteComposer";
import { RouteResults } from "./components/RouteResults";
import { SourceRail } from "./components/SourceRail";
import { loadPlanner } from "./planner/bridge";
import { buildInfo } from "./release/buildInfo";
import type { PlannerRequest, PlannerSnapshot, RecommendationObjective } from "./planner/types";

const initialRequest: PlannerRequest = {
  assetSymbol: "USDC",
  destinationId: "ledger-arbitrum",
  amount: "1000.00",
  objective: "BALANCED",
  snapshotAgeSeconds: 14,
};

const defaultsByAsset: Record<string, Pick<PlannerRequest, "destinationId" | "amount">> = {
  USDC: { destinationId: "ledger-arbitrum", amount: "1000.00" },
  BTC: { destinationId: "cold-bitcoin", amount: "0.02" },
  ETH: { destinationId: "trezor-ethereum", amount: "1.00" },
  XRP: { destinationId: "kraken-xrp", amount: "1000" },
};

const navItems = [
  ["route", "Planner", "active"],
  ["wallet", "Destinations", "4"],
  ["catalog", "Catalog", ""],
  ["shield", "Security", ""],
] as const;

export function App() {
  const [request, setRequest] = useState(initialRequest);
  const [snapshot, setSnapshot] = useState<PlannerSnapshot | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showBuildInfo, setShowBuildInfo] = useState(false);
  const evaluationId = useRef(0);
  const desktopRuntime = "__TAURI_INTERNALS__" in window;

  async function refresh(next: PlannerRequest) {
    const currentEvaluation = ++evaluationId.current;
    setBusy(true);
    setError(null);
    try {
      const nextSnapshot = await loadPlanner(next);
      if (currentEvaluation === evaluationId.current) setSnapshot(nextSnapshot);
    } catch (reason) {
      if (currentEvaluation === evaluationId.current) {
        setSnapshot(null);
        setError(reason instanceof Error ? reason.message : "Planner evaluation failed");
      }
    } finally {
      if (currentEvaluation === evaluationId.current) setBusy(false);
    }
  }

  useEffect(() => {
    void refresh(initialRequest);
    return () => { evaluationId.current += 1; };
  }, []);

  function selectAsset(assetSymbol: string) {
    const defaults = defaultsByAsset[assetSymbol] ?? defaultsByAsset.USDC;
    const next = { ...request, assetSymbol, ...defaults };
    setRequest(next);
    void refresh(next);
  }

  function selectObjective(objective: RecommendationObjective) {
    const next = { ...request, objective };
    setRequest(next);
    void refresh(next);
  }

  const selectedDestination = snapshot?.destinations.find(
    (destination) => destination.id === snapshot.selection.destinationId,
  );
  const pendingChanges = snapshot
    ? request.assetSymbol !== snapshot.selection.assetSymbol
      || request.destinationId !== snapshot.selection.destinationId
      || request.amount !== snapshot.selection.amount
      || request.objective !== snapshot.selection.objective
    : false;
  const stale = snapshot ? snapshot.snapshotAgeSeconds > snapshot.freshForSeconds : false;
  const fixture = snapshot?.mode === "OFFLINE_FIXTURE";

  return (
    <div className="app-shell">
      <nav className="app-nav" aria-label="Primary navigation">
        <div className="brand"><span className="brand-mark"><Icon name="rail" size={24}/></span><div><strong>{buildInfo.productName}</strong><small>{buildInfo.distributionKind === "community" ? "Unofficial source build" : "Destination-aware routing"}</small></div></div>
        <div className="nav-list">{navItems.map(([icon, label, suffix]) => <button key={label} disabled title={suffix === "active" ? "Current workspace" : "Not available in this offline fixture build"} className={suffix === "active" ? "is-active" : ""} aria-current={suffix === "active" ? "page" : undefined}><Icon name={icon}/><span>{label}</span>{suffix && suffix !== "active" && <em>{suffix}</em>}</button>)}</div>
        <div className="nav-bottom"><div className="local-device"><span className={`device-light ${desktopRuntime ? "" : "is-preview"}`}/><div><strong>Local planner</strong><small>{desktopRuntime ? "Mac · desktop bridge" : "Browser preview · no desktop bridge"}</small></div></div><button type="button" className="build-info-trigger" aria-label="Build and source information" onClick={() => setShowBuildInfo(true)}><span className="device-light"/><div><strong>{buildInfo.distributionLabel}</strong><small>Source · license · identity</small></div></button><button type="button" className="avatar" aria-label="Workspace profile" title="Not available in this offline fixture build" disabled>RS</button></div>
      </nav>

      <main className="main-plane">
        <header className="topbar">
          <div><span className="eyebrow">Offline planning environment</span><h1>Route workspace</h1></div>
          <div className="topbar-actions"><span className={`fresh-status ${stale ? "is-stale" : ""}`}><Icon name={stale ? "clock" : "check"} size={14}/>{fixture ? (stale ? "Fixture outside simulated freshness window" : "Fixture within simulated freshness window") : (stale ? "Snapshot stale" : "Snapshot current")}</span><span className="authority-status"><Icon name="shield" size={15}/><strong>Local authority</strong></span></div>
        </header>

        {error && <div className="error-banner" role="alert"><Icon name="info"/><div><strong>Planner unavailable</strong><p>{error}</p></div><button onClick={() => void refresh(request)}>Retry</button></div>}

        {snapshot ? <>
          <div className="snapshot-strip"><span><span className="fixture-dot"/> {snapshot.mode.replaceAll("_", " ")}</span><p><strong>{snapshot.provider}</strong><span/>{fixture ? "Fixture timestamp" : "Snapshot"} {snapshot.generatedAt.replace("T", " · ").replace("Z", " UTC")}</p><span className="snapshot-age">{fixture ? `Simulated age ${snapshot.snapshotAgeSeconds}s` : `${snapshot.snapshotAgeSeconds}s old`}</span></div>
          <div className="workspace-grid">
            <SourceRail balances={snapshot.balances} selected={request.assetSymbol} onSelect={selectAsset}/>
            <div className="route-workspace">
              <RouteComposer request={request} destinations={snapshot.destinations} busy={busy} onChange={setRequest} onObjective={selectObjective} onCompare={() => void refresh(request)}/>
              {!pendingChanges
                ? <RouteResults routes={snapshot.eligible} excluded={snapshot.excluded} asset={snapshot.selection.assetSymbol} destination={selectedDestination}/>
                : <div className="draft-results" role="status"><Icon name="info"/><div><strong>Route recommendation hidden for this draft</strong><p>Edit complete route inputs, then compare again.</p></div></div>}
              <div className="planner-boundary"><Icon name="lock" size={17}/><p><strong>Planning only.</strong> No exchange credential is connected and no withdrawal can be submitted from this build.</p></div>
            </div>
            <EvidenceRail snapshot={snapshot} destination={selectedDestination} desktopRuntime={desktopRuntime}/>
          </div>
        </> : busy ? <div className="loading-workspace" aria-live="polite"><span/><span/><span/><p>Loading the local planner…</p></div>
          : error ? <div className="draft-results" role="status"><Icon name="info"/><div><strong>No planner result is available</strong><p>Resolve the error above or retry the request.</p></div></div>
            : null}
      </main>
      {showBuildInfo && <BuildInformation info={buildInfo} onClose={() => setShowBuildInfo(false)}/>}
    </div>
  );
}
