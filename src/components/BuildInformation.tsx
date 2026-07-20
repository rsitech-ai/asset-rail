import { useEffect, useRef } from "react";
import type { BuildInfo } from "../release/buildInfo";

export function BuildInformation({ info, onClose }: { info: BuildInfo; onClose: () => void }) {
  const dialogRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };

    dialogRef.current?.focus();
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("keydown", closeOnEscape);
      previousFocus?.focus({ preventScroll: true });
    };
  }, [onClose]);

  return (
    <div className="build-info-backdrop" role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget) onClose();
    }}>
      <section ref={dialogRef} tabIndex={-1} className="build-info-dialog" role="dialog" aria-modal="true" aria-labelledby="build-info-title">
        <div className="build-info-heading">
          <div><span className="eyebrow">Source and distribution</span><h2 id="build-info-title">Build and source information</h2></div>
          <button type="button" aria-label="Close build information" onClick={onClose}>Close</button>
        </div>
        <p className="distribution-label">{info.distributionLabel}</p>
        <dl className="build-info-grid">
          <div><dt>Product</dt><dd>{info.productName}</dd></div>
          <div><dt>Version / build</dt><dd>{info.version} / {info.buildNumber}</dd></div>
          <div><dt>Source revision</dt><dd><code>{info.sourceRevision}</code></dd></div>
          <div><dt>Source license</dt><dd><code>{info.sourceLicenseId}</code><span>{info.sourceLicenseStatus}</span></dd></div>
        </dl>
        {info.sourceUrl
          ? <a className="source-link" href={info.sourceUrl} target="_blank" rel="noreferrer">Open source corresponding to this build</a>
          : <p className="source-unavailable">No durable source URL was recorded for this local build.</p>}
        <div className="trademark-boundary">
          <strong>Brand and service boundary</strong>
          <p>Official names, icons, and services are not granted by a source-code license. Modified distributions must use their own identity, credentials, support, and privacy disclosures.</p>
        </div>
        <p className="security-contact-boundary">Report vulnerabilities privately through GitHub Security Advisories or info@rsitech.ai. Do not file public issues for suspected vulnerabilities.</p>
      </section>
    </div>
  );
}
