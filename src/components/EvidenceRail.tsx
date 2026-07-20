import { Icon } from "./Icon";
import type { DestinationView, PlannerSnapshot } from "../planner/types";

function readableConfidence(value?: string) { return value?.toLowerCase().replaceAll("_", " ") ?? "Unavailable"; }

export function EvidenceRail({ snapshot, destination, desktopRuntime }: { snapshot: PlannerSnapshot; destination?: DestinationView; desktopRuntime: boolean }) {
  const fixture = snapshot.mode === "OFFLINE_FIXTURE";
  const checks = [
    ["Canonical chain", destination?.chainId ?? "Not selected"],
    [fixture ? "Fixture destination" : "Destination evidence", fixture && destination ? "Simulated verification evidence" : readableConfidence(destination?.confidence)],
    ["Asset support", destination?.supportedAssets.join(", ") || "None declared"],
    ["Memo / tag", destination?.memo ? `Present · ${destination.memo}` : "Not required"],
  ];
  return (
    <aside className="evidence-rail" aria-label="Destination evidence">
      <div className="evidence-title"><span className="eyebrow">Independent checks</span><h2>Destination evidence</h2><p>{desktopRuntime ? "The local Rust core applies these facts before cost ranking." : "The browser preview mirrors the deterministic fixture checks before cost ranking."}</p></div>
      <div className="evidence-checks">{checks.map(([label, value]) => <div className="evidence-check" key={label}><span className="check-mark"><Icon name="check" size={13}/></span><div><small>{label}</small><strong>{value}</strong></div></div>)}</div>
      <div className="freshness-card"><div><Icon name="clock" size={17}/><span>{fixture ? "Simulated fixture freshness" : "Quote freshness"}</span></div><strong>{fixture ? `Simulated age ${snapshot.snapshotAgeSeconds}s` : `${snapshot.snapshotAgeSeconds}s old`}</strong><div className="freshness-meter"><span style={{ width: `${Math.min(100, (snapshot.snapshotAgeSeconds / snapshot.freshForSeconds) * 100)}%` }} /></div><small>Execution would require an immediate live refresh.</small></div>
      <div className="authority-note"><Icon name="shield" size={20}/><div><strong>{desktopRuntime ? "Local final authority" : "Preview boundary"}</strong><p>{desktopRuntime ? "Cloud and JavaScript cannot mark a blocked route eligible." : "Preview results are illustrative; the desktop Rust core remains the release authority."}</p></div></div>
    </aside>
  );
}
