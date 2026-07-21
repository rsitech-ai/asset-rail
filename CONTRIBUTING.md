# Contributing to AssetRail

AssetRail welcomes focused fixes and improvements to the offline planner,
security boundaries, tests, documentation, and community build. The current
project is deliberately not accepting live trading, withdrawal execution,
credential UI, or real-wallet fixtures.

## Before opening work

- Search existing issues and pull requests.
- Use a public issue for reproducible bugs or a concise feature proposal.
- Use the private process in [SECURITY.md](SECURITY.md) for suspected
  vulnerabilities.
- Never attach API keys, secrets, private wallet addresses, production logs, or
  personal financial data.

Maintainers may close proposals that expand the money-moving or credential
surface without an approved security design.

## Development setup

Install the pinned toolchains listed in [README.md](README.md), then run:

```bash
npm ci
npm run test:run
npm run test:config
npm run build
npm run verify:security
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features -- -D warnings
```

GitHub Actions verifies source and secret history on pull requests. Contributors
should still report the local commands they ran and their results in the pull
request, especially for packaging or Apple signing work that CI does not cover.

## Change requirements

- Keep React presentation and Rust authority boundaries explicit.
- Validate every IPC input; TypeScript types are not runtime validation.
- Use decimal strings across IPC and exact arithmetic in financial logic.
- Keep fixtures synthetic. Do not add real addresses, memos, account labels,
  credentials, provider responses, or user data.
- Add a failing regression test before changing behavior, then show the test
  passing with the implementation.
- Keep dependencies pinned and justify each new dependency.
- Update documentation when behavior, commands, support, or limitations change.
- Do not weaken CSP, Tauri capabilities, redaction, source disclosure, or the
  distinct community-build identity to make a test pass.

## Developer Certificate of Origin

AssetRail uses the [Developer Certificate of Origin 1.1](DCO.md), not a CLA.
Sign off every commit with:

```bash
git commit -s
```

The sign-off certifies that you have the right to submit the contribution under
the applicable project license. It is not a copyright assignment. Contributions
made with code-generation or other tools remain subject to the same rights and
provenance requirements.

## Pull requests

A pull request should contain:

- the problem and intended outcome;
- the smallest coherent implementation;
- tests that cover the changed behavior and important failure path;
- exact local verification results;
- documentation and security impact;
- screenshots for visible UI changes;
- an explicit statement when the change affects compatibility or release scope.

The maintainer may request smaller commits, additional boundary tests, or a
design discussion before accepting security-sensitive changes. Merge and
release authority are defined in [GOVERNANCE.md](GOVERNANCE.md).
