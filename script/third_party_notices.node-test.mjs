import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  collectRuntimePackageIds,
  renderThirdPartyNotices,
  releaseResourceMap,
} from "./third_party_notices.mjs";

test("runtime package traversal excludes dev and build-only dependency edges", () => {
  const metadata = {
    resolve: {
      nodes: [
        {
          id: "root",
          deps: [
            { pkg: "runtime", dep_kinds: [{ kind: null }] },
            { pkg: "dev", dep_kinds: [{ kind: "dev" }] },
            { pkg: "build", dep_kinds: [{ kind: "build" }] },
          ],
        },
        { id: "runtime", deps: [{ pkg: "transitive", dep_kinds: [{ kind: null }] }] },
        { id: "transitive", deps: [] },
        { id: "dev", deps: [] },
        { id: "build", deps: [] },
      ],
    },
  };

  assert.deepEqual(collectRuntimePackageIds(metadata, "root"), ["runtime", "transitive"]);
});

test("third-party notice rendering is sorted, source-bound, and path-free", () => {
  const rendered = renderThirdPartyNotices({
    revision: "0123456789abcdef0123456789abcdef01234567",
    rustPackages: [
      { name: "zeta", version: "2.0.0", license: "MIT", licenseTexts: ["MIT text"] },
      { name: "alpha", version: "1.0.0", license: "Apache-2.0", licenseTexts: ["Apache text"] },
    ],
    npmPackages: [
      { name: "react", version: "19.2.7", license: "MIT", licenseTexts: ["React license"] },
    ],
  });

  assert.match(rendered, /Source revision: 0123456789abcdef0123456789abcdef01234567/);
  assert.ok(rendered.indexOf("alpha 1.0.0") < rendered.indexOf("zeta 2.0.0"));
  assert.match(rendered, /react 19\.2\.7/);
  assert.match(rendered, /Apache text/);
  assert.doesNotMatch(rendered, /\/Users\/|\/home\/|\\Users\\/);
});

test("release bundles include project and dependency legal resources", () => {
  assert.deepEqual(releaseResourceMap, {
    "PrivacyInfo.xcprivacy": "PrivacyInfo.xcprivacy",
    "../COPYRIGHT": "COPYRIGHT",
    "../LICENSE": "LICENSE",
    "../NOTICE": "NOTICE",
    "../TRADEMARKS.md": "TRADEMARKS.md",
    "target/release-resources/THIRD_PARTY_NOTICES.txt": "THIRD_PARTY_NOTICES.txt",
  });
});

test("cargo-about template preserves license text without HTML escaping", async () => {
  const template = await readFile(new URL("third_party_licenses.hbs", import.meta.url), "utf8");
  assert.match(template, /\{\{\{text\}\}\}/);
  assert.match(template, /\{\{\{name\}\}\}/);
  assert.doesNotMatch(template, /(^|[^\{])\{\{text\}\}([^\}]|$)/);
});
