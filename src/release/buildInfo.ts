export type DistributionKind = "development" | "official" | "community";

export interface BuildInfo {
  distributionKind: DistributionKind;
  distributionLabel: string;
  productName: string;
  sourceLicenseId: string;
  sourceLicenseStatus: string;
  sourceRevision: string;
  sourceUrl: string | null;
  version: string;
  buildNumber: string;
}

type BuildEnvironment = Record<string, string | undefined>;

const distributionLabels: Record<DistributionKind, string> = {
  community: "Unofficial community build",
  development: "Local source build",
  official: "Official distribution",
};

const fullRevision = /^[0-9a-f]{40}$/;

export function createBuildInfo(environment: BuildEnvironment): BuildInfo {
  const distributionKind = environment.VITE_DISTRIBUTION_KIND ?? "development";
  if (!(distributionKind in distributionLabels)) throw new Error(`Unknown distribution kind: ${distributionKind}`);

  const kind = distributionKind as DistributionKind;
  const productName = environment.VITE_PRODUCT_NAME ?? "AssetRail";
  if (kind === "community" && /assetrail/i.test(productName)) {
    throw new Error("Community builds require a distinct product name that does not use AssetRail as the primary name");
  }

  const sourceRevision = environment.VITE_SOURCE_REVISION ?? "unrecorded";
  if (kind === "official" && !fullRevision.test(sourceRevision)) {
    throw new Error("Official builds require a full lowercase Git commit as the source revision");
  }

  const sourceUrl = environment.VITE_SOURCE_URL || null;
  if (sourceUrl && !sourceUrl.startsWith("https://")) throw new Error("Build metadata requires an HTTPS source URL");

  return {
    distributionKind: kind,
    distributionLabel: distributionLabels[kind],
    productName,
    sourceLicenseId: environment.VITE_SOURCE_LICENSE_ID ?? "MPL-2.0",
    sourceLicenseStatus: environment.VITE_SOURCE_LICENSE_STATUS ?? "Open-source license adopted",
    sourceRevision,
    sourceUrl,
    version: environment.VITE_APP_VERSION ?? "0.1.0",
    buildNumber: environment.VITE_BUILD_NUMBER ?? "development",
  };
}

export const buildInfo = createBuildInfo(import.meta.env as BuildEnvironment);
