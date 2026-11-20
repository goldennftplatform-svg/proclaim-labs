# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
cd "$BASE/proclaim-labs" || exit 99
L="$BASE/logs/offline-cache-check.txt"
{
echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
for w in lp0001-phase0 lp0021-phase0; do
  echo "############ $w ############"
  echo "--- Cargo.lock package/version pairs ---"
  awk '/^name = /{n=$3} /^version = /{gsub(/"/,"",n); print n"-"$3}' "$w/Cargo.lock" | sort
  echo "--- cached .crate files matching those names ---"
  for spec in $(awk '/^name = /{n=$3} /^version = /{gsub(/"/,"",n); print n"-"$3}' "$w/Cargo.lock" | sort -u); do
    name=${spec%-*}
    ver=${spec##*-}
    # name may contain hyphens; use version suffix from spec
    f=$(find "$HOME/.cargo/registry/cache" -type f -name "${spec}.crate" 2>/dev/null | head -1)
    if [ -n "$f" ]; then echo "CACHED $spec"; else echo "NOT-CACHED $spec"; fi
  done
done
} 2>&1 | tee "$L"
