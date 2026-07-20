import { releaseResourceMap } from "./third_party_notices.mjs";

const officialBundleId = "ai.rsitech.assetrail";
const bundleIdPattern = /^[A-Za-z0-9]+(?:[.-][A-Za-z0-9]+)+$/;
const fullRevisionPattern = /^[0-9a-f]{40}$/;
const defaultSourceRepository = "https://github.com/rsitech-ai/asset-rail";
const productNamePattern = /^[A-Za-z0-9][A-Za-z0-9 ._-]{1,63}$/;
const forbiddenHostPathFragments = ["/Users/", "/home/", "\\Users\\"];

export function findEmbeddedHostPath(contents) {
  const text = Buffer.isBuffer(contents) ? contents.toString("latin1") : String(contents);
  return forbiddenHostPathFragments.find((fragment) => text.includes(fragment)) ?? null;
}

export function resolveCommunityIdentity(environment) {
  const productName = environment.COMMUNITY_PRODUCT_NAME || "Rail Planner Community";
  const bundleId = environment.COMMUNITY_BUNDLE_ID || "org.example.railplanner.community";

  if (/assetrail/i.test(productName)) throw new Error("Community product name must be distinct from AssetRail");
  if (!productNamePattern.test(productName)) throw new Error("COMMUNITY_PRODUCT_NAME contains unsupported characters");
  if (!bundleIdPattern.test(bundleId)) throw new Error("COMMUNITY_BUNDLE_ID must be a reverse-DNS-style identifier");
  if (bundleId.toLowerCase() === officialBundleId) throw new Error("Community bundle ID must differ from the official bundle ID");

  return { bundleId, productName };
}

export function createCommunityConfig({ bundleId, productName }) {
  return {
    productName,
    identifier: bundleId,
    app: { windows: [{ label: "main", title: productName }] },
    bundle: {
      icon: [
        "icons/community/32x32.png",
        "icons/community/128x128.png",
        "icons/community/128x128@2x.png",
        "icons/community/icon.icns",
      ],
      resources: releaseResourceMap,
    },
  };
}

export function createCommunityBuildPlan({ environment, revision, dirty }) {
  if (!fullRevisionPattern.test(revision)) {
    throw new Error("Community builds require a full lowercase Git commit");
  }
  if (dirty) throw new Error("Community builds require a clean working tree");

  const buildHome = environment.HOME;
  if (typeof buildHome !== "string" || !buildHome.startsWith("/") || buildHome === "/") {
    throw new Error("Community builds require an absolute non-root HOME for path remapping");
  }

  const identity = resolveCommunityIdentity(environment);
  const sourceRepository = environment.COMMUNITY_SOURCE_REPOSITORY || defaultSourceRepository;
  if (!sourceRepository.startsWith("https://")) {
    throw new Error("COMMUNITY_SOURCE_REPOSITORY must use HTTPS");
  }

  return {
    identity,
    bundlePath: `src-tauri/target/release/bundle/macos/${identity.productName}.app`,
    environment: {
      ...environment,
      CARGO_ENCODED_RUSTFLAGS: [
        environment.CARGO_ENCODED_RUSTFLAGS,
        `--remap-path-prefix=${buildHome}=/build`,
      ].filter(Boolean).join("\u001f"),
      CARGO_PROFILE_RELEASE_DEBUG: "false",
      CARGO_PROFILE_RELEASE_STRIP: "symbols",
      VITE_APP_VERSION: "0.1.1",
      VITE_BUILD_NUMBER: revision.slice(0, 12),
      VITE_DISTRIBUTION_KIND: "community",
      VITE_PRODUCT_NAME: identity.productName,
      VITE_SOURCE_LICENSE_ID: "Apache-2.0",
      VITE_SOURCE_LICENSE_STATUS: "Open-source license adopted",
      VITE_SOURCE_REVISION: revision,
      VITE_SOURCE_URL: `${sourceRepository.replace(/\/$/, "")}/tree/${revision}`,
    },
    arguments: ["build", "--bundles", "app", "--config", JSON.stringify(createCommunityConfig(identity)), "--", "--locked"],
  };
}
