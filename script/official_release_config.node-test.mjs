import assert from "node:assert/strict";
import test from "node:test";
import {
  createOfficialBuildPlan,
  expectedOfficialIdentity,
  findEmbeddedHostPath,
} from "./official_release_config.mjs";

test("official build plan binds v0.1.2 to an exact clean source revision", () => {
  const revision = "0123456789abcdef0123456789abcdef01234567";
  const plan = createOfficialBuildPlan({
    environment: { HOME: "/Users/tester" },
    revision,
    dirty: false,
  });

  assert.equal(plan.identity, expectedOfficialIdentity);
  assert.equal(plan.bundlePath, "src-tauri/target/release/bundle/macos/AssetRail.app");
  assert.equal(plan.environment.APPLE_SIGNING_IDENTITY, expectedOfficialIdentity);
  assert.equal(plan.environment.VITE_APP_VERSION, "0.1.2");
  assert.equal(plan.environment.VITE_BUILD_NUMBER, "2");
  assert.equal(plan.environment.VITE_DISTRIBUTION_KIND, "official");
  assert.equal(plan.environment.VITE_PRODUCT_NAME, "AssetRail");
  assert.equal(plan.environment.VITE_SOURCE_LICENSE_ID, "Apache-2.0");
  assert.equal(plan.environment.VITE_SOURCE_REVISION, revision);
  assert.equal(
    plan.environment.VITE_SOURCE_URL,
    `https://github.com/rsitech-ai/asset-rail/tree/${revision}`,
  );
  assert.deepEqual(plan.arguments.slice(0, 3), ["build", "--bundles", "app"]);
  const configIndex = plan.arguments.indexOf("--config") + 1;
  const config = JSON.parse(plan.arguments[configIndex]);
  assert.equal(config.bundle.resources["../LICENSE"], "LICENSE");
  assert.equal(config.bundle.resources["../NOTICE"], "NOTICE");
  assert.equal(
    config.bundle.resources["target/release-resources/THIRD_PARTY_NOTICES.txt"],
    "THIRD_PARTY_NOTICES.txt",
  );
  assert.deepEqual(plan.arguments.slice(-2), ["--", "--locked"]);
});

test("official builds reject dirty or ambiguous source state", () => {
  assert.throws(
    () => createOfficialBuildPlan({ environment: { HOME: "/Users/tester" }, revision: "main", dirty: false }),
    /full lowercase Git commit/,
  );
  assert.throws(
    () => createOfficialBuildPlan({
      environment: { HOME: "/Users/tester" },
      revision: "0123456789abcdef0123456789abcdef01234567",
      dirty: true,
    }),
    /clean working tree/,
  );
});

test("official build preparation fails closed when notarization credentials are present", () => {
  for (const key of [
    "APPLE_ID",
    "APPLE_PASSWORD",
    "APPLE_API_ISSUER",
    "APPLE_API_KEY",
    "APPLE_API_KEY_PATH",
  ]) {
    assert.throws(
      () => createOfficialBuildPlan({
        environment: { HOME: "/Users/tester", [key]: "configured" },
        revision: "0123456789abcdef0123456789abcdef01234567",
        dirty: false,
      }),
      /notarization credentials/i,
    );
  }
});

test("official executable verification rejects embedded workstation paths", () => {
  assert.equal(findEmbeddedHostPath(Buffer.from("safe release bytes")), null);
  assert.equal(findEmbeddedHostPath(Buffer.from("debug info: /Users/example/source.rs")), "/Users/");
  assert.equal(findEmbeddedHostPath(Buffer.from("debug info: /home/example/source.rs")), "/home/");
  assert.equal(findEmbeddedHostPath(Buffer.from("debug info: C:\\Users\\example\\source.rs")), "\\Users\\");
});
