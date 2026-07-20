# Public/private and service boundary

## Public candidate

- React/TypeScript UI, Rust route kernel, tests, schemas, mock fixtures, build
  scripts, security design, public configuration, and community artwork.
- No server repository is present. The meaningful public build is the offline
  fixture planner.
- Third-party libraries remain under upstream licenses.

## Private or owner-controlled

- Apple certificates, private keys, profiles, Team access, App Store Connect
  credentials, release approvals, and signing/notarization operations.
- Future exchange API credentials and any production user data.
- Official hosted services, if introduced later; none are reachable today.
- Official trademarks, bundle identity, icon, domains, and Apple distribution
  records. The public security and support channels are documented in
  `SECURITY.md` and `SUPPORT.md`.

## Runtime proof

The shipping command surface registers one offline planner IPC command. Tauri
capabilities are empty; there is no shell, filesystem, deep link, remote window,
generic HTTP plugin, telemetry, crash upload, persistence, credential UI, live
connector, or withdrawal execution path. The fixed-origin Binance transport and
Keychain vault are future foundations and are not reachable from the WebView.

Community builds use their own name, icon, and bundle identifier and require no
official Apple identity or production credential. Any future live connector,
hosted API, analytics, updates, purchases, or account feature must reopen this
boundary and the privacy manifest before it becomes reachable.
