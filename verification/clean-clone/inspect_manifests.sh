# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
cd "$BASE/proclaim-labs" || exit 99
for w in lp0001-phase0 lp0021-permissions lp0021-phase0 lp0021-phase1 lp0021-phase2-testimonials lp0021-phase3 lp0021-phase4-provider-protocol lp0023-claim-binding; do
  echo "############ WORKSPACE: $w ############"
  echo "--- $w/Cargo.toml ---"
  cat "$w/Cargo.toml"
  echo "--- crate manifests ---"
  find "$w" -name Cargo.toml -not -path "$w/Cargo.toml" -print | sort | while read -r m; do
    echo "===== $m ====="
    cat "$m"
  done
  echo "--- cargo registry cache availability ---"
  echo "CARGO_HOME=${CARGO_HOME:-$HOME/.cargo}"
  ls "$HOME/.cargo/registry" 2>&1 | head -20 || true
  echo
done
