# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
source ~/.cargo/env
export CARGO_BUILD_JOBS=2
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
LOGDIR="$BASE/logs"
mkdir -p "$LOGDIR"
L="$LOGDIR/starter-checks2.log"
cd "$BASE/proclaim-module-starter" || exit 99

NODE=""
for cand in node "/mnt/c/Program Files/nodejs/node.exe"; do
  if command -v "$cand" >/dev/null 2>&1; then NODE="$cand"; break; fi
done
[ -n "$NODE" ] || NODE="(not found)"

{
echo "############ REPO: proclaim-module-starter (corrected env) ############"
echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "HEAD: $(git rev-parse HEAD)"
echo "rustc: $(rustc --version 2>&1)"
echo "cargo: $(cargo --version 2>&1)"
echo "node candidate: $NODE"
if [ "$NODE" != "(not found)" ]; then echo "node version: $('$NODE' --version 2>&1)"; fi
echo
echo "--- README has no self-contained test command. Commands it documents all require external prerequisites: ---"
echo "  * npm install ./clients/js   (requires a checkout of upstream proclaim repo - NOT in this clone)"
echo "  * node search.mjs            (requires a running registry-api on 127.0.0.1:3030 - NOT started)"
echo "  * cargo run --example register_and_search (requires copying Cargo.toml.template -> Cargo.toml and pinned upstream git deps)"
echo "  * lgs / Logos Basecamp toolchain / disposable sequencer (lgs MISSING; no chain activity in scope)"
echo
echo "--- COMMAND: cargo metadata --manifest-path Cargo.toml.template --no-deps ---"
cargo metadata --manifest-path Cargo.toml.template --no-deps 2>&1
echo "EXIT_CODE cargo_metadata_template: $?"
echo
echo "--- COMMAND: node --check clients/js/search.mjs (self-contained syntax check) ---"
if [ "$NODE" != "(not found)" ]; then
  "$NODE" --check clients/js/search.mjs 2>&1
  echo "EXIT_CODE node_check: $?"
else
  echo "SKIPPED: node not available"
fi
echo
echo "--- COMMAND: python3 -c compile src/lib.rs? not applicable (Rust). Instead: check authored files exist and non-empty ---"
find . -path ./.git -prune -o -type f \( -name '*.rs' -o -name '*.mjs' -o -name '*.qml' -o -name '*.cpp' -o -name '*.h' -o -name 'Cargo.toml.template' \) -print0 | sort -z | xargs -0 -r wc -c
echo
echo "--- RESULT.md referenced by README line 107 ---"
[ -f RESULT.md ] && echo "RESULT.md PRESENT" || echo "RESULT.md ABSENT"
echo "=== END ==="
} 2>&1 | tee "$L"
