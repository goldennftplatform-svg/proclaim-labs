# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
cd "$BASE/proclaim-labs" || exit 99
L="$BASE/logs/offline-cache-check2.txt"
{
echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
for w in lp0001-phase0 lp0021-phase0; do
  echo "############ $w ############"
  specs=$(awk -F'"' '/^name = /{n=$2} /^version = /{print n"-"$2}' "$w/Cargo.lock" | grep -v '^-$' | sort -u)
  cached=0; missing=0
  for spec in $specs; do
    f=$(find "$HOME/.cargo/registry/cache" -type f -name "${spec}.crate" 2>/dev/null | head -1)
    if [ -n "$f" ]; then echo "CACHED    $spec"; cached=$((cached+1)); else echo "NOT-CACHED $spec"; missing=$((missing+1)); fi
  done
  echo "SUMMARY $w cached=$cached missing=$missing"
done
} 2>&1 | tee "$L"
