#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import process from "node:process";
import { createCommunityBuildPlan, findEmbeddedHostPath } from "./community_config.mjs";
import { generateThirdPartyNotices } from "./third_party_notices.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function git(...arguments_) {
  return execFileSync("git", arguments_, { cwd: repositoryRoot, encoding: "utf8" }).trim();
}

const revision = git("rev-parse", "HEAD");
const dirty = git("status", "--porcelain=v1", "--untracked-files=all").length > 0;
const systemFirstEnvironment = {
  ...process.env,
  PATH: `/usr/bin:/bin:/usr/sbin:/sbin:${process.env.PATH || ""}`,
};
const plan = createCommunityBuildPlan({ environment: systemFirstEnvironment, revision, dirty });
const tauri = path.join(repositoryRoot, "node_modules", ".bin", "tauri");

const notices = await generateThirdPartyNotices({ repositoryRoot, revision });
process.stdout.write(`Generated notices for ${notices.rustPackageCount} Rust and ${notices.npmPackageCount} npm runtime packages\n`);
process.stdout.write(`Building ${plan.identity.productName} (${plan.identity.bundleId}) from ${revision}\n`);
const result = spawnSync(tauri, plan.arguments, {
  cwd: repositoryRoot,
  env: plan.environment,
  stdio: "inherit",
});

if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);

const bundlePath = path.join(repositoryRoot, plan.bundlePath);
const executableName = execFileSync("/usr/libexec/PlistBuddy", ["-c", "Print :CFBundleExecutable", path.join(bundlePath, "Contents", "Info.plist")], { encoding: "utf8" }).trim();
const embeddedHostPath = findEmbeddedHostPath(readFileSync(path.join(bundlePath, "Contents", "MacOS", executableName)));
if (embeddedHostPath) {
  throw new Error(`Community executable contains forbidden host path marker: ${embeddedHostPath}`);
}
for (const resource of ["COPYRIGHT", "LICENSE", "NOTICE", "TRADEMARKS.md", "THIRD_PARTY_NOTICES.txt", "PrivacyInfo.xcprivacy"]) {
  if (!existsSync(path.join(bundlePath, "Contents", "Resources", resource))) {
    throw new Error(`Community bundle is missing legal resource: ${resource}`);
  }
}
const seal = spawnSync("/usr/bin/codesign", ["--force", "--deep", "--sign", "-", bundlePath], { stdio: "inherit" });
if (seal.error) throw seal.error;
if (seal.status !== 0) process.exit(seal.status ?? 1);

const verify = spawnSync("/usr/bin/codesign", ["--verify", "--deep", "--strict", "--verbose=2", bundlePath], { stdio: "inherit" });
if (verify.error) throw verify.error;
process.exitCode = verify.status ?? 1;
