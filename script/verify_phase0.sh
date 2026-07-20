#!/usr/bin/env bash
set -euo pipefail

required=(
  docs/adr/0001-rust-security-kernel.md
  docs/adr/0002-connector-and-validator-registries.md
  docs/adr/0003-keychain-and-sqlcipher.md
  docs/adr/0004-credential-modes.md
  docs/adr/0005-price-evidence.md
  docs/adr/0006-signed-desktop-updates.md
  docs/security/threat-model.md
  docs/security/authorization-matrix.md
  docs/security/redaction-contract.md
  docs/security/security-owner-checklist.md
  docs/runbooks/provider-outage.md
  docs/runbooks/credential-revocation.md
  docs/runbooks/storage-recovery.md
  docs/runbooks/release-and-rollback.md
  schemas/v1/README.md
)

for path in "${required[@]}"; do
  if [[ ! -s "$path" ]]; then
    echo "missing or empty Phase 0 artifact: $path" >&2
    exit 1
  fi
done

echo "Phase 0 artifact inventory passed (${#required[@]} files)."
