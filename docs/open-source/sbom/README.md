# SBOM release assets

AssetRail publishes three machine-readable inventories with each source
release. Generated SBOMs are release assets rather than tracked source files so
their source revision can match the immutable release commit exactly.

| File | Format | Scope |
| --- | --- | --- |
| `assetrail-source.cdx.json` | CycloneDX JSON | Source tree excluding Git data, build output, and generated SBOMs |
| `assetrail-source.spdx.json` | SPDX JSON | Same source-tree scan |
| `assetrail-npm.cdx.json` | CycloneDX JSON | Installed npm graph from the exact lockfile |

The generator requires Syft 1.44.0 and fails on another version. Obtain Syft
from its official release, verify the release checksum independently, then run
from a clean release commit:

```bash
npm ci
SYFT_BIN=/absolute/path/to/syft ./script/generate_sbom.sh
```

The default output directory is `dist/sbom/`. Set `SBOM_OUTPUT_DIR` to another
absolute or repository-relative directory when preparing release assets.

The generator:

- binds source inventories to `git rev-parse HEAD`;
- excludes `.git`, `dist`, Rust targets, and previous generated SBOMs;
- normalizes only the exact repository-root prefix;
- rejects macOS, Linux, or Windows workstation home paths;
- prints SHA-256 hashes for all generated JSON files.

Record the Syft version, Syft archive checksum, package-lock hash, Cargo-lock
hash, component counts, and generated-file hashes in the GitHub release notes.
These inventories do not replace vulnerability review or the upstream notices
required when distributing a compiled application.
