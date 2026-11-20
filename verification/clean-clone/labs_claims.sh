# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
cd "$BASE/proclaim-labs" || exit 99
L="$BASE/logs/labs-claims.txt"
{
echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
for w in lp0001-phase0 lp0021-permissions lp0021-phase0 lp0021-phase1 lp0021-phase2-testimonials lp0021-phase3 lp0021-phase4-provider-protocol lp0023-claim-binding; do
  echo "############ $w ############"
  echo "--- README.md claimed tests/commands ---"
  grep -nE 'tests? passing|[0-9]+ tests|cargo fmt|cargo clippy|cargo test|offline|SPDX' "$w/README.md" 2>/dev/null | head -30 || echo "no README.md"
  echo "--- RESULT.md claimed results ---"
  grep -nE 'tests? passing|[0-9]+ passed|[0-9]+ tests|cargo fmt|cargo clippy|cargo test|offline|EXIT|exit code' "$w/RESULT.md" 2>/dev/null | head -40 || echo "no RESULT.md"
  echo
done
} 2>&1 | tee "$L"
