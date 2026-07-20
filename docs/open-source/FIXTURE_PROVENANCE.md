# Fixture provenance and privacy

The planner demo is deterministic, non-secret, and not connected to an exchange
or wallet. Destination values are deliberately non-address tokens with the
prefix `synthetic:`; they cannot be mistaken for user wallet or exchange deposit
data. Destination labels say `Synthetic fixture`, and all memos/tags are absent.

Asset, chain, and token identifiers represent public protocol metadata used to
exercise mapping behavior. Financial values are invented test inputs. The
fixture must never gain a real account, wallet address, memo/tag, user label,
credential, or provider response. Tests in `src/planner/demo.test.ts` and
`src-tauri/tests/planner_snapshot.rs` enforce the destination rule.
