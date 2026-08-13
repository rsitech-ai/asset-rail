#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import process from "node:process";
import {
  createOfficialBuildPlan,
  expectedOfficialIdentity,
  findEmbeddedHostPath,
} from "./official_release_config.mjs";
import { generateThirdPartyNotices } from "./third_party_notices.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function git(...arguments_) {
  return execFileSync("git", arguments_, { cwd: repositoryRoot, encoding: "utf8" }).trim();
}

function run(command, arguments_, options = {}) {
  const result = spawnSync(command, arguments_, {
    cwd: repositoryRoot,
    encoding: options.capture ? "utf8" : undefined,
    stdio: options.capture ? "pipe" : "inherit",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const details = options.capture ? `${result.stdout || ""}${result.stderr || ""}`.trim() : "";
    throw new Error(`${command} failed${details ? `: ${details}` : ""}`);
  }
  return options.capture ? `${result.stdout || ""}${result.stderr || ""}` : "";
}

const revision = git("rev-parse", "HEAD");
const dirty = git("status", "--porcelain=v1", "--untracked-files=all").length > 0;
const systemFirstEnvironment = {
  ...process.env,
  PATH: `/usr/bin:/bin:/usr/sbin:/sbin:${process.env.PATH || ""}`,
};
const plan = createOfficialBuildPlan({ environment: systemFirstEnvironment, revision, dirty });
const identities = execFileSync("/usr/bin/security", ["find-identity", "-v", "-p", "codesigning"], { encoding: "utf8" });
if (!identities.includes(`\"${expectedOfficialIdentity}\"`)) {
  throw new Error(`Missing usable signing identity: ${expectedOfficialIdentity}`);
}

const tauri = path.join(repositoryRoot, "node_modules", ".bin", "tauri");
const notices = await generateThirdPartyNotices({ repositoryRoot, revision });
process.stdout.write(`Generated notices for ${notices.rustPackageCount} Rust and ${notices.npmPackageCount} npm runtime packages\n`);
process.stdout.write(`Building official AssetRail 0.1.2 (2) from ${revision}\n`);
const result = spawnSync(tauri, plan.arguments, {
  cwd: repositoryRoot,
  env: plan.environment,
  stdio: "inherit",
});
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);

const bundlePath = path.join(repositoryRoot, plan.bundlePath);
const infoPlist = path.join(bundlePath, "Contents", "Info.plist");
const executableName = execFileSync("/usr/libexec/PlistBuddy", ["-c", "Print :CFBundleExecutable", infoPlist], { encoding: "utf8" }).trim();
const executablePath = path.join(bundlePath, "Contents", "MacOS", executableName);
const embeddedHostPath = findEmbeddedHostPath(readFileSync(executablePath));
if (embeddedHostPath) throw new Error(`Official executable contains forbidden host path marker: ${embeddedHostPath}`);

run("/usr/bin/codesign", ["--verify", "--deep", "--strict", "--verbose=2", bundlePath]);
const signature = run("/usr/bin/codesign", ["-dvv", bundlePath], { capture: true });
if (!signature.includes(`Authority=${expectedOfficialIdentity}`)) throw new Error("Official bundle has the wrong signing authority");
if (!signature.includes("TeamIdentifier=2NY8A789TN")) throw new Error("Official bundle has the wrong Apple Team ID");
if (!/flags=.*runtime/.test(signature)) throw new Error("Official bundle is missing the hardened runtime flag");

const identifier = execFileSync("/usr/libexec/PlistBuddy", ["-c", "Print :CFBundleIdentifier", infoPlist], { encoding: "utf8" }).trim();
const version = execFileSync("/usr/libexec/PlistBuddy", ["-c", "Print :CFBundleShortVersionString", infoPlist], { encoding: "utf8" }).trim();
const build = execFileSync("/usr/libexec/PlistBuddy", ["-c", "Print :CFBundleVersion", infoPlist], { encoding: "utf8" }).trim();
if (identifier !== "ai.rsitech.assetrail" || version !== "0.1.2" || build !== "2") {
  throw new Error(`Unexpected bundle metadata: ${identifier} ${version} (${build})`);
}
for (const resource of ["COPYRIGHT", "LICENSE", "NOTICE", "TRADEMARKS.md", "THIRD_PARTY_NOTICES.txt", "PrivacyInfo.xcprivacy"]) {
  if (!existsSync(path.join(bundlePath, "Contents", "Resources", resource))) {
    throw new Error(`Official bundle is missing legal resource: ${resource}`);
  }
}

const architectures = run("/usr/bin/lipo", ["-archs", executablePath], { capture: true }).trim();
process.stdout.write(`Verified ${bundlePath}\nSource ${revision}\nArchitecture ${architectures}\nSigning team 2NY8A789TN\n`);
