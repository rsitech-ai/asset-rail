import assert from "node:assert/strict";
import test from "node:test";
import {
  createCommunityBuildPlan,
  createCommunityConfig,
  findEmbeddedHostPath,
  resolveCommunityIdentity,
} from "./community_config.mjs";

test("uses a distinct safe community identity by default", () => {
  const identity = resolveCommunityIdentity({});
  assert.deepEqual(identity, {
    bundleId: "org.example.railplanner.community",
    productName: "Rail Planner Community",
  });

  const config = createCommunityConfig(identity);
  assert.equal(config.identifier, identity.bundleId);
  assert.equal(config.productName, identity.productName);
  assert.equal(config.app.windows[0].title, identity.productName);
  assert.deepEqual(config.bundle.icon, [
    "icons/community/32x32.png",
    "icons/community/128x128.png",
    "icons/community/128x128@2x.png",
    "icons/community/icon.icns",
  ]);
});

test("rejects official or malformed community identity", () => {
  for (const env of [
    { COMMUNITY_PRODUCT_NAME: "AssetRail" },
    { COMMUNITY_PRODUCT_NAME: "assetrail community" },
    { COMMUNITY_BUNDLE_ID: "ai.rsitech.assetrail" },
    { COMMUNITY_BUNDLE_ID: "not a bundle id" },
  ]) {
    assert.throws(() => resolveCommunityIdentity(env));
  }
});

test("community build plan binds a clean build to an exact source revision", () => {
  const revision = "0123456789abcdef0123456789abcdef01234567";
  const plan = createCommunityBuildPlan({
    environment: { HOME: "/Users/tester" },
    revision,
    dirty: false,
  });

  assert.equal(plan.environment.VITE_DISTRIBUTION_KIND, "community");
  assert.equal(plan.environment.VITE_PRODUCT_NAME, "Rail Planner Community");
  assert.equal(plan.environment.VITE_SOURCE_REVISION, revision);
  assert.equal(plan.environment.VITE_SOURCE_URL, `https://github.com/rsitech-ai/asset-rail/tree/${revision}`);
  assert.equal(plan.environment.VITE_SOURCE_LICENSE_ID, "MPL-2.0");
  assert.equal(plan.environment.VITE_SOURCE_LICENSE_STATUS, "Open-source license adopted");
  assert.equal(plan.environment.VITE_BUILD_NUMBER, revision.slice(0, 12));
  assert.equal(plan.environment.CARGO_PROFILE_RELEASE_DEBUG, "false");
  assert.equal(plan.environment.CARGO_PROFILE_RELEASE_STRIP, "symbols");
  assert.equal(
    plan.environment.CARGO_ENCODED_RUSTFLAGS,
    "--remap-path-prefix=/Users/tester=/build",
  );
  assert.deepEqual(plan.arguments.slice(0, 3), ["build", "--bundles", "app"]);
  assert.deepEqual(plan.arguments.slice(-2), ["--", "--locked"]);
  const configIndex = plan.arguments.indexOf("--config") + 1;
  assert.equal(JSON.parse(plan.arguments[configIndex]).identifier, "org.example.railplanner.community");
  assert.equal(
    plan.bundlePath,
    "src-tauri/target/release/bundle/macos/Rail Planner Community.app",
  );
});

test("community builds reject dirty or ambiguous source state", () => {
  assert.throws(
    () => createCommunityBuildPlan({ environment: {}, revision: "0123456789abcdef0123456789abcdef01234567", dirty: true }),
    /clean working tree/,
  );
  assert.throws(
    () => createCommunityBuildPlan({ environment: {}, revision: "main", dirty: false }),
    /full lowercase Git commit/,
  );
});

test("community bundle verification rejects embedded workstation paths", () => {
  assert.equal(findEmbeddedHostPath(Buffer.from("safe release bytes")), null);
  assert.equal(findEmbeddedHostPath(Buffer.from("debug info: /Users/example/source.rs")), "/Users/");
  assert.equal(findEmbeddedHostPath(Buffer.from("debug info: /home/example/source.rs")), "/home/");
  assert.equal(findEmbeddedHostPath(Buffer.from("debug info: C:\\Users\\example\\source.rs")), "\\Users\\");
});
