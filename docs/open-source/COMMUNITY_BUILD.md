# Community build

Status: the clean-worktree build, strict ad-hoc signature, source disclosure,
fixture workflow, and host-path rejection were verified on macOS on 2026-07-20.
Repeat the same checks at the exact commit distributed by a fork.

Requirements: macOS 13+, Xcode 26.6 command-line tools, Node 24.10.0, npm
11.6.2, and Rust 1.91.0. No Apple Developer membership, official signing
identity, provisioning profile, Team ID, or production credential is required.

```bash
npm ci
npm run test:run
npm run build:community
```

Output:

```text
src-tauri/target/release/bundle/macos/Rail Planner Community.app
```

The command requires a clean Git worktree, records the full source commit and
durable commit URL in the frontend, uses `org.example.railplanner.community`,
uses community artwork, builds with both locks, then ad-hoc seals and strictly
verifies the assembled app. It does not notarize or grant an official identity.
Release debug information is disabled, symbols are stripped, and the builder
fails before signing if the executable still embeds a macOS, Linux, or Windows
home-directory path marker.

The 2026-07-20 rehearsal installed the locked graph, reported zero npm
vulnerabilities, passed the frontend suite, built the arm64 app, passed strict
signature verification, launched the real bundle, rendered a usable planner,
and displayed the full source revision plus MPL-2.0 status in-app.

Optional safe overrides:

```bash
COMMUNITY_PRODUCT_NAME='My Rail Planner' \
COMMUNITY_BUNDLE_ID='com.example.myrailplanner' \
COMMUNITY_SOURCE_REPOSITORY='https://github.com/example/my-rail-planner' \
npm run build:community
```

The product name may not contain `AssetRail`, the bundle identifier may not be
the official identifier, and the source repository must use HTTPS. A publicly
distributed fork must also replace artwork, use its own Apple account and
services, publish accurate privacy/support information, identify itself as
unofficial, describe material changes, and comply with MPL-2.0 plus all
applicable third-party licenses and notices.
