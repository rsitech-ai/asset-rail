import { execFileSync } from "node:child_process";
import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";

const licenseFilePattern = /^(?:COPYING|COPYRIGHT|LICEN[CS]E|NOTICE)(?:[._-].*)?$/i;
const forbiddenHostPathFragments = ["/Users/", "/home/", "\\Users\\"];

export const releaseResourceMap = Object.freeze({
  "PrivacyInfo.xcprivacy": "PrivacyInfo.xcprivacy",
  "../COPYRIGHT": "COPYRIGHT",
  "../LICENSE": "LICENSE",
  "../NOTICE": "NOTICE",
  "../TRADEMARKS.md": "TRADEMARKS.md",
  "target/release-resources/THIRD_PARTY_NOTICES.txt": "THIRD_PARTY_NOTICES.txt",
});

export function collectRuntimePackageIds(metadata, rootId) {
  const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
  const found = new Set();
  const pending = [rootId];

  while (pending.length > 0) {
    const current = pending.pop();
    const node = nodes.get(current);
    if (!node) throw new Error(`Cargo metadata is missing resolve node: ${current}`);

    for (const dependency of node.deps) {
      const isRuntime = dependency.dep_kinds.some((kind) => kind.kind === null);
      if (!isRuntime || dependency.pkg === rootId || found.has(dependency.pkg)) continue;
      found.add(dependency.pkg);
      pending.push(dependency.pkg);
    }
  }

  return [...found].sort();
}

function renderPackageSection(kind, packages) {
  const title = `${kind} dependencies`;
  const sections = [title, "=".repeat(title.length), ""];
  for (const dependency of [...packages].sort((left, right) => {
    return `${left.name}@${left.version}`.localeCompare(`${right.name}@${right.version}`);
  })) {
    sections.push(`${dependency.name} ${dependency.version}`);
    sections.push(`SPDX license expression: ${dependency.license || "not declared"}`);
    dependency.licenseTexts.forEach((licenseText, index) => {
      sections.push(`License text ${index + 1}:`);
      sections.push(licenseText.trim());
    });
    sections.push("");
  }
  return sections.join("\n");
}

export function renderThirdPartyNotices({ revision, rustPackages = [], rustNoticeText = null, npmPackages }) {
  const rendered = [
    "AssetRail third-party notices",
    "================================",
    "",
    `Source revision: ${revision}`,
    "Target: aarch64-apple-darwin official and community application bundles",
    "",
    "The packages below retain their upstream licenses. This inventory is generated",
    "from the exact Cargo and npm runtime dependency graphs used for the bundle.",
    "",
    rustNoticeText ?? renderPackageSection("Rust", rustPackages),
    renderPackageSection("npm", npmPackages),
    "",
  ].join("\n");

  const forbiddenPath = forbiddenHostPathFragments.find((fragment) => rendered.includes(fragment));
  if (forbiddenPath) throw new Error(`Third-party notices contain forbidden host path marker: ${forbiddenPath}`);
  return rendered;
}

async function readLicenseTexts(packageDirectory, explicitLicenseFile) {
  const candidates = new Set();
  for (const entry of await readdir(packageDirectory, { withFileTypes: true })) {
    if (entry.isFile() && licenseFilePattern.test(entry.name)) {
      candidates.add(path.join(packageDirectory, entry.name));
    }
  }
  if (explicitLicenseFile) candidates.add(explicitLicenseFile);

  const texts = [];
  for (const candidate of [...candidates].sort()) {
    try {
      if (!(await stat(candidate)).isFile()) continue;
      const text = (await readFile(candidate, "utf8")).replaceAll("\r\n", "\n").trim();
      if (text && !texts.includes(text)) texts.push(text);
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
  }
  return texts;
}

export async function generateThirdPartyNotices({ repositoryRoot, revision }) {
  const cargoManifest = path.join(repositoryRoot, "src-tauri", "Cargo.toml");
  const cargoMetadata = JSON.parse(execFileSync("cargo", [
    "metadata",
    "--manifest-path", cargoManifest,
    "--locked",
    "--filter-platform", "aarch64-apple-darwin",
    "--format-version", "1",
  ], { cwd: repositoryRoot, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 }));
  const rootPackage = cargoMetadata.packages.find((dependency) => {
    return dependency.name === "assetrail" && dependency.source === null;
  });
  if (!rootPackage) throw new Error("Cargo metadata does not identify the AssetRail root package");

  const rustPackageIds = collectRuntimePackageIds(cargoMetadata, rootPackage.id);
  const outputDirectory = path.join(repositoryRoot, "src-tauri", "target", "release-resources");
  await mkdir(outputDirectory, { recursive: true });
  const rustNoticesPath = path.join(outputDirectory, "RUST_THIRD_PARTY_LICENSES.txt");
  execFileSync("cargo", [
    "about",
    "generate",
    "--manifest-path", cargoManifest,
    "--config", path.join(repositoryRoot, "src-tauri", "about.toml"),
    "--frozen",
    "--fail",
    "--output-file", rustNoticesPath,
    path.join(repositoryRoot, "script", "third_party_licenses.hbs"),
  ], { cwd: repositoryRoot, stdio: "inherit" });
  const rustNoticeText = await readFile(rustNoticesPath, "utf8");

  const npmQuery = JSON.parse(execFileSync("npm", ["query", ".prod", "--json"], {
    cwd: repositoryRoot,
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  }));
  const npmPackages = [];
  for (const dependency of npmQuery.filter((candidate) => candidate.name !== "assetrail")) {
    const licenseTexts = await readLicenseTexts(dependency.path, null);
    if (licenseTexts.length === 0) throw new Error(`npm dependency has no bundled license text: ${dependency.name} ${dependency.version}`);
    npmPackages.push({
      name: dependency.name,
      version: dependency.version,
      license: dependency.license,
      licenseTexts,
    });
  }

  const outputPath = path.join(outputDirectory, "THIRD_PARTY_NOTICES.txt");
  const rendered = renderThirdPartyNotices({ revision, rustNoticeText, npmPackages });
  await writeFile(outputPath, rendered, "utf8");
  return { outputPath, rustPackageCount: rustPackageIds.length, npmPackageCount: npmPackages.length };
}
