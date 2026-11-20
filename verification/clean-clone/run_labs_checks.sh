# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
source ~/.cargo/env
export CARGO_BUILD_JOBS=2

BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
LOGDIR="$BASE/logs"
mkdir -p "$LOGDIR"
cd "$BASE/proclaim-labs" || exit 99

WORKSPACES="lp0001-phase0 lp0021-permissions lp0021-phase0 lp0021-phase1 lp0021-phase2-testimonials lp0021-phase3 lp0021-phase4-provider-protocol lp0023-claim-binding"

SUMMARY="$LOGDIR/labs-summary.txt"
: > "$SUMMARY"
echo "date_utc_start: $(date -u '+%Y-%m-%dT%H:%M:%SZ')" >> "$SUMMARY"
echo "rustc: $(rustc --version)" >> "$SUMMARY"
echo "cargo: $(cargo --version)" >> "$SUMMARY"
echo "cargo_fmt: $(cargo fmt --version 2>&1)" >> "$SUMMARY"
echo "cargo_clippy: $(cargo clippy --version 2>&1)" >> "$SUMMARY"
echo "CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS" >> "$SUMMARY"
echo "cargo_home: ${CARGO_HOME:-$HOME/.cargo}" >> "$SUMMARY"
echo "cargo registry cache top-level:" >> "$SUMMARY"
ls -1 "$HOME/.cargo/registry" 2>&1 >> "$SUMMARY"
echo "cargo registry cached crate count: $(find "$HOME/.cargo/registry/cache" -type f -name '*.crate' 2>/dev/null | wc -l)" >> "$SUMMARY"
echo "" >> "$SUMMARY"

for w in $WORKSPACES; do
  L="$LOGDIR/labs-$w.log"
  echo "############ WORKSPACE: $w ############" | tee "$L"
  echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')" | tee -a "$L"
  echo "cwd: $BASE/proclaim-labs/$w" | tee -a "$L"
  cd "$BASE/proclaim-labs/$w" || exit 99

  echo "############ COMMAND: cargo fmt --all -- --check ############" | tee -a "$L"
  cargo fmt --all -- --check 2>&1 | tee -a "$L"
  rc_fmt=${PIPESTATUS[0]}
  echo "############ EXIT_CODE fmt: $rc_fmt ############" | tee -a "$L"

  echo "############ COMMAND: cargo clippy --workspace --all-targets -- -D warnings ############" | tee -a "$L"
  cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee -a "$L"
  rc_clippy=${PIPESTATUS[0]}
  echo "############ EXIT_CODE clippy: $rc_clippy ############" | tee -a "$L"

  echo "############ COMMAND: cargo test --workspace --offline ############" | tee -a "$L"
  cargo test --workspace --offline 2>&1 | tee -a "$L"
  rc_test=${PIPESTATUS[0]}
  echo "############ EXIT_CODE test: $rc_test ############" | tee -a "$L"

  passed=$(grep -oE 'test result: ok\. [0-9]+ passed' "$L" | grep -oE '[0-9]+ passed' | awk '{s+=$1} END {print s+0}')
  failed=$(grep -oE 'test result: FAILED\. [0-9]+ failed' "$L" | grep -oE '[0-9]+ failed' | awk '{s+=$1} END {print s+0}')
  echo "############ TEST COUNT passed=$passed failed=$failed ############" | tee -a "$L"
  echo "$w | fmt=$rc_fmt | clippy=$rc_clippy | test_offline=$rc_test | passed=$passed | failed=$failed | log=logs/labs-$w.log" >> "$SUMMARY"
  echo "" >> "$SUMMARY"
done

echo "date_utc_end: $(date -u '+%Y-%m-%dT%H:%M:%SZ')" >> "$SUMMARY"
echo "=== CLEANUP target/ DIRS ===" | tee -a "$LOGDIR/labs-cleanup.log"
cd "$BASE/proclaim-labs" || exit 99
find . -type d -name target -prune -print | tee -a "$LOGDIR/labs-cleanup.log"
find . -type d -name target -prune -exec rm -rf {} + 2>&1 | tee -a "$LOGDIR/labs-cleanup.log"
echo "remaining target dirs: $(find . -type d -name target | wc -l)" | tee -a "$LOGDIR/labs-cleanup.log"

echo "=== SUMMARY ==="
cat "$SUMMARY"
