# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
LOGDIR="$BASE/logs"
mkdir -p "$LOGDIR"
L="$LOGDIR/starter-checks.log"
cd "$BASE/proclaim-module-starter" || exit 99

{
echo "############ REPO: proclaim-module-starter ############"
echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "HEAD: $(git rev-parse HEAD)"
echo "HEAD_DATE: $(git log -1 --format='%cI %s')"
echo
echo "--- Documented verification commands found in README.md / clients/js/README.md / docs/INTEGRATION-CHECKLIST.md ---"
grep -nE 'npm install|node search|cargo run|cargo test|cargo fmt|cargo clippy|bash |\.sh|make api|RESULT\.md|Quick orientation' README.md clients/js/README.md docs/INTEGRATION-CHECKLIST.md || true
echo
echo "--- Is there a self-contained test/verify script in this repo? ---"
find . -path ./.git -prune -o -type f \( -name '*.sh' -o -name 'Makefile' -o -name 'Justfile' -o -name 'package.json' -o -name 'Cargo.toml' \) -print | sort || true
echo "(none listed above = no self-contained test script / real Cargo.toml / package.json)"
echo
echo "--- RESULT.md referenced by README but present? ---"
if [ -f RESULT.md ]; then echo "RESULT.md: PRESENT"; else echo "RESULT.md: ABSENT (README.md line 107 references it)"; fi
echo
echo "--- Tool availability ---"
for t in node npm cargo python3; do p=$(command -v "$t" 2>/dev/null) || p=MISSING; echo "$t: $p"; done
node --version 2>&1 || true
npm --version 2>&1 || true
echo
echo "--- COMMAND: node --check clients/js/search.mjs (self-contained syntax check) ---"
node --check clients/js/search.mjs 2>&1
echo "EXIT_CODE node_check: $?"
echo
echo "--- COMMAND: cargo metadata --manifest-path Cargo.toml.template --no-deps (expect failure: template, not a real manifest) ---"
cargo metadata --manifest-path Cargo.toml.template --no-deps 2>&1
echo "EXIT_CODE cargo_metadata_template: $?"
echo
echo "--- README-declared external prerequisites check ---"
echo "lgs: $(command -v lgs 2>/dev/null || echo MISSING)"
echo "upstream proclaim repo present in clone: $([ -d ../proclaim ] && echo yes || echo no)"
echo "running registry-api on 127.0.0.1:3030: NOT STARTED (out of scope: no deployment / no chain)"
echo "=== END ==="
} 2>&1 | tee "$L"
