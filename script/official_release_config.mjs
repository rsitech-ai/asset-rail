import { releaseResourceMap } from "./third_party_notices.mjs";

const fullRevisionPattern = /^[0-9a-f]{40}$/;
const forbiddenHostPathFragments = ["/Users/", "/home/", "\\Users\\"];
const notarizationCredentialKeys = [
  "APPLE_ID",
  "APPLE_PASSWORD",
  "APPLE_API_ISSUER",
  "APPLE_API_KEY",
  "APPLE_API_KEY_PATH",
];

export const expectedOfficialIdentity = "Developer ID Application: Rafal Sikora (2NY8A789TN)";

export function findEmbeddedHostPath(contents) {
  const text = Buffer.isBuffer(contents) ? contents.toString("latin1") : String(contents);
  return forbiddenHostPathFragments.find((fragment) => text.includes(fragment)) ?? null;
}

export function createOfficialBuildPlan({ environment, revision, dirty }) {
  if (!fullRevisionPattern.test(revision)) {
    throw new Error("Official builds require a full lowercase Git commit");
  }
  if (dirty) throw new Error("Official builds require a clean working tree");

  const buildHome = environment.HOME;
  if (typeof buildHome !== "string" || !buildHome.startsWith("/") || buildHome === "/") {
    throw new Error("Official builds require an absolute non-root HOME for path remapping");
  }

  if (notarizationCredentialKeys.some((key) => environment[key])) {
    throw new Error("Official build preparation refuses notarization credentials; notarization requires a separate approved step");
  }

  const sourceRepository = "https://github.com/rsitech-ai/asset-rail";
  return {
    identity: expectedOfficialIdentity,
    bundlePath: "src-tauri/target/release/bundle/macos/AssetRail.app",
    environment: {
      ...environment,
      APPLE_SIGNING_IDENTITY: expectedOfficialIdentity,
      CARGO_ENCODED_RUSTFLAGS: [
        environment.CARGO_ENCODED_RUSTFLAGS,
        `--remap-path-prefix=${buildHome}=/build`,
      ].filter(Boolean).join("\u001f"),
      CARGO_PROFILE_RELEASE_DEBUG: "false",
      CARGO_PROFILE_RELEASE_STRIP: "symbols",
      VITE_APP_VERSION: "0.1.1",
      VITE_BUILD_NUMBER: "1",
      VITE_DISTRIBUTION_KIND: "official",
      VITE_PRODUCT_NAME: "AssetRail",
      VITE_SOURCE_LICENSE_ID: "Apache-2.0",
      VITE_SOURCE_LICENSE_STATUS: "Apache-2.0 open-source release",
      VITE_SOURCE_REVISION: revision,
      VITE_SOURCE_URL: `${sourceRepository}/tree/${revision}`,
    },
    arguments: [
      "build",
      "--bundles",
      "app",
      "--config",
      JSON.stringify({ bundle: { resources: releaseResourceMap } }),
      "--",
      "--locked",
    ],
  };
}
