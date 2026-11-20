#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
set -euo pipefail
cd "$(dirname "$0")"
for file in crates/wallet-chain/src/lib.rs crates/wallet-cli/src/approval.rs \
            crates/wallet-cli/src/args.rs crates/wallet-cli/src/consent.rs \
            crates/wallet-cli/src/dispatch.rs crates/wallet-cli/src/runtime.rs \
            crates/wallet-cli/tests/e2e.rs; do
    printf '%-44s %s\n' "$file" "$(grep -c '#\[test\]' "$file")"
done
printf 'TOTAL %s\n' "$(grep -rh '#\[test\]' crates --include='*.rs' | wc -l)"
