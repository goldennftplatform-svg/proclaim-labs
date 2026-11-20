# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
LOGDIR="$BASE/logs"
mkdir -p "$LOGDIR"
L="$LOGDIR/scan-all.log"
OVERALL="$LOGDIR/scan-summary.txt"
: > "$L"
: > "$OVERALL"

echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')" | tee -a "$L"
echo "scan method: git ls-files (tracked files only), grep -n, case-sensitive unless noted" | tee -a "$L"

for r in proclaim-verify proclaim-labs proclaim-module-starter; do
  echo "############ SECRET + SPDX SCAN: $r ############" | tee -a "$L"
  cd "$BASE/$r" || exit 99
  echo "HEAD: $(git rev-parse HEAD)" | tee -a "$L"
  tracked=$(git ls-files | wc -l)
  echo "tracked_files_total: $tracked" | tee -a "$L"

  for pat in 'ghp_' 'github_pat_' 'PRIVATE KEY' 'private key'; do
    echo "--- pattern (case-sensitive literal): $pat ---" | tee -a "$L"
    git ls-files -z | xargs -0 -r grep -nF "$pat" 2>/dev/null | tee -a "$L"
    rc=${PIPESTATUS[0]}
    n=$(git ls-files -z | xargs -0 -r grep -lF "$pat" 2>/dev/null | wc -l)
    echo "count_files_matching: $n   grep_rc=$rc (1 = no matches)" | tee -a "$L"
  done

  echo "--- SPDX on authored source files ---" | tee -a "$L"
  echo "definition: tracked text files with extensions .rs .toml .sh .md .mjs .js .py .qml .cpp .h .cmake, files named CMakeLists.txt, plus extensionless scripts in bin/ and tests/. Excludes Cargo.lock (generated), LICENSE-* (standard legal texts), and all .git content." | tee -a "$L"
  git ls-files -z | while IFS= read -r -d '' f; do
    case "$f" in
      *.rs|*.toml|*.sh|*.md|*.mjs|*.js|*.py|*.qml|*.cpp|*.h|*.cmake|CMakeLists.txt|bin/*|tests/*) printf '%s\n' "$f" ;;
    esac
  done | sort > "$LOGDIR/authored-$r.txt"
  authored_total=$(wc -l < "$LOGDIR/authored-$r.txt")
  ok_count=0
  missing_list=""
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    if grep -qF 'SPDX-License-Identifier: MIT AND Apache-2.0' "$f" 2>/dev/null; then
      ok_count=$((ok_count+1))
    else
      missing_list="$missing_list\n$f"
    fi
  done < "$LOGDIR/authored-$r.txt"
  echo "authored_source_files: $authored_total" | tee -a "$L"
  echo "with_SPDX_MIT_AND_Apache-2.0: $ok_count" | tee -a "$L"
  echo "without_SPDX:$missing_list" | tee -a "$L"
  echo "$r | tracked=$tracked | authored=$authored_total | spdx_ok=$ok_count | ghp_=$(git ls-files -z | xargs -0 -r grep -lF 'ghp_' 2>/dev/null | wc -l) | github_pat_=$(git ls-files -z | xargs -0 -r grep -lF 'github_pat_' 2>/dev/null | wc -l) | PRIVATE_KEY_upper=$(git ls-files -z | xargs -0 -r grep -lF 'PRIVATE KEY' 2>/dev/null | wc -l) | private_key_lower=$(git ls-files -z | xargs -0 -r grep -lF 'private key' 2>/dev/null | wc -l)" >> "$OVERALL"
  echo | tee -a "$L"
done

echo "=== SCAN SUMMARY ===" | tee -a "$L"
cat "$OVERALL" | tee -a "$L"
