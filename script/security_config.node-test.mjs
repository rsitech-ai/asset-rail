import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../", import.meta.url);

async function read(relativePath) {
  return readFile(new URL(relativePath, root), "utf8");
}

async function readJson(relativePath) {
  return JSON.parse(await read(relativePath));
}

async function exists(relativePath) {
  try {
    await read(relativePath);
    return true;
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }
}

test("Tauri grants the main window no unused built-in permissions", async () => {
  const capability = await readJson("src-tauri/capabilities/default.json");
  const config = await readJson("src-tauri/tauri.conf.json");

  assert.deepEqual(capability.windows, ["main"]);
  assert.deepEqual(capability.permissions, []);
  assert.deepEqual(config.app.security.capabilities, ["default"]);
});

test("release toolchains and clean installs are pinned", async () => {
  const manifest = await readJson("package.json");
  const readme = await read("README.md");
  const toolchain = await read("rust-toolchain.toml");

  assert.equal(manifest.packageManager, "npm@11.6.2");
  assert.deepEqual(manifest.engines, {
    node: ">=24.10.0 <25",
    npm: "11.6.2",
  });
  assert.deepEqual(manifest.devEngines, {
    runtime: { name: "node", version: "24.10.0", onFail: "error" },
    packageManager: { name: "npm", version: "11.6.2", onFail: "error" },
  });
  assert.match(readme, /```bash\s+npm ci\s+/);
  assert.doesNotMatch(readme, /```bash\s+npm install\s+/);
  assert.match(toolchain, /channel = "1\.91\.0"/);
  assert.match(toolchain, /components = \["clippy", "rustfmt"\]/);
});

test("repository-local secrets and IDE state stay untracked", async () => {
  const ignore = await read(".gitignore");

  for (const pattern of [
    ".env",
    ".env.*",
    "!.env.example",
    ".vscode/",
    ".idea/",
    "*.key",
    "*.pem",
    "*.p8",
    "*.p12",
    "*.mobileprovision",
  ]) {
    assert.ok(ignore.split("\n").includes(pattern), `missing ignore pattern: ${pattern}`);
  }
});

test("build helper validates mode before building and never kills by process name", async () => {
  const runner = await read("script/build_and_run.sh");
  const validation = runner.indexOf('case "$MODE" in');
  const build = runner.indexOf("npm run tauri build -- --debug");

  assert.ok(validation >= 0, "mode validation is missing");
  assert.ok(build > validation, "mode must be validated before the build starts");
  assert.doesNotMatch(runner, /\bpkill\b/);
  assert.doesNotMatch(runner, /\/usr\/bin\/open\s+-n\b/);
  assert.match(runner, /app_pids\(\)/);
  assert.match(runner, /stop_app_instances\(\)/);
  assert.match(runner, /\/bin\/kill\s+"\$pid"/);
  assert.match(runner, /APP_BINARY/);
});

test("dependency updates and local secret scanning remain enforced while CI is excluded", async () => {
  for (const path of [
    ".github/dependabot.yml",
    ".gitleaks.toml",
    "script/verify_security_surface.sh",
  ]) {
    assert.equal(await exists(path), true, `missing security automation: ${path}`);
  }

  assert.equal(await exists(".github/workflows/ci.yml"), false, "CI must remain outside this release scope");

  const dependabot = await read(".github/dependabot.yml");
  for (const ecosystem of ["npm", "cargo"]) {
    assert.match(dependabot, new RegExp(`package-ecosystem: ["']${ecosystem}["']`));
  }
  assert.doesNotMatch(dependabot, /package-ecosystem: ["']github-actions["']/);
});

test("secret-scan exceptions are value-specific rather than test-tree exclusions", async () => {
  const config = await read(".gitleaks.toml");

  assert.match(config, /credentialFingerprint:\\s\*"0123456789abcdef"/);
  for (const match of config.matchAll(/paths\s*=\s*\[([\s\S]*?)\]/g)) {
    assert.doesNotMatch(match[1], /(^|\/)(src\/|tests?\/)/);
  }
});

test("public documentation is licensed, operational, and free of private release artifacts", async () => {
  const authorization = await read("docs/security/authorization-matrix.md");
  const redaction = await read("docs/security/redaction-contract.md");
  const schemas = await read("schemas/v1/README.md");
  const readme = await read("README.md");
  const releasing = await read("docs/RELEASING.md");
  const security = await read("SECURITY.md");
  const contributing = await read("CONTRIBUTING.md");
  const copyright = await read("COPYRIGHT");
  const notice = await read("NOTICE");
  const license = await read("LICENSE");
  const licenseScope = await read("LICENSES/README.md");
  const maintainers = await read("MAINTAINERS.md");
  const changelog = await read("CHANGELOG.md");
  const manifest = await readJson("package.json");
  const tauriConfig = await readJson("src-tauri/tauri.conf.json");
  const cargoManifest = await read("src-tauri/Cargo.toml");
  const sbomGenerator = await read("script/generate_sbom.sh");

  assert.match(authorization, /`core:default` is forbidden/);
  assert.match(redaction, /Current foundation coverage/);
  assert.match(redaction, /Release checks must/);
  assert.doesNotMatch(schemas, /## Planned envelopes/);
  assert.match(schemas, /must pass secret-pattern scans before release/);
  assert.match(readme, /License: Apache-2\.0/);
  assert.doesNotMatch(readme, /not yet licensed|must not be published/i);
  assert.match(releasing, /Developer ID-signed direct-download release/);
  assert.match(releasing, /CI is not used as a gate/);
  assert.match(security, /GitHub Private Vulnerability Reporting/);
  assert.match(security, /info@rsitech\.ai/);
  assert.match(contributing, /Developer Certificate of Origin 1\.1/);
  assert.equal(
    copyright,
    "Copyright 2026 Rafal Sikora\n\nAssetRail is maintained publicly by RSI Tech.\nSee LICENSES/README.md for license scope and third-party attribution.\n",
  );
  assert.match(notice, /Copyright 2026 Rafal Sikora\./);
  assert.match(notice, /Apache License, Version 2\.0/);
  assert.match(license, /^\s*Apache License\s+Version 2\.0, January 2004/m);
  assert.match(licenseScope, /original\s+source code, tests, scripts, schemas, and project documentation/);
  assert.equal(await exists("LICENSES/CC-BY-4.0.txt"), false);
  assert.match(maintainers, /RSI Tech/);
  assert.match(maintainers, /https:\/\/rsitech\.ai/);
  assert.match(maintainers, /info@rsitech\.ai/);
  assert.match(changelog, /## \[0\.1\.1\] - 2026-07-20/);
  assert.match(changelog, /Historical `v0\.1\.0` license grants remain unchanged/);
  assert.equal(manifest.version, "0.1.1");
  assert.equal(manifest.license, "Apache-2.0");
  assert.equal(tauriConfig.version, "0.1.1");
  assert.match(cargoManifest, /^version = "0\.1\.1"$/m);
  assert.match(cargoManifest, /^authors = \["RSI Tech <info@rsitech\.ai>"\]$/m);
  assert.match(cargoManifest, /^license = "Apache-2\.0"$/m);
  assert.match(sbomGenerator, /SBOM_OUTPUT_DIR:-\$ROOT_DIR\/dist\/sbom/);

  for (const generatedSbom of [
    "docs/open-source/sbom/assetrail-npm.cdx.json",
    "docs/open-source/sbom/assetrail-source.cdx.json",
    "docs/open-source/sbom/assetrail-source.spdx.json",
  ]) {
    assert.equal(await exists(generatedSbom), false, `generated SBOM must be a release asset: ${generatedSbom}`);
  }

  for (const privateArtifact of [
    "binance-withdrawal-router-production-spec.md",
    "plans/active/2026-07-15-assetrail-open-source-release.md",
    "docs/superpowers/plans/2026-07-14-assetrail-live-read-only-connectors.md",
    "docs/release/0.1.0/RELEASE_STATUS.md",
    "script/build_app_store.sh",
  ]) {
    assert.equal(await exists(privateArtifact), false, `private release artifact remains: ${privateArtifact}`);
  }
});
