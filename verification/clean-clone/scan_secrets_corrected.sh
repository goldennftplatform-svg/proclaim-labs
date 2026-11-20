# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
LOGDIR="$BASE/logs"
L="$LOGDIR/scan-secrets-corrected.log"
: > "$L"
echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')" | tee -a "$L"
echo "method: for each tracked file (git ls-files), grep -n -F pattern; count = files with >=1 match" | tee -a "$L"
for r in proclaim-verify proclaim-labs proclaim-module-starter; do
  cd "$BASE/$r" || exit 99
  echo "############ $r (HEAD $(git rev-parse HEAD)) ############" | tee -a "$L"
  for pat in 'ghp_' 'github_pat_' 'PRIVATE KEY' 'private key'; do
    files=0; hits=0
    while IFS= read -r f; do
      [ -z "$f" ] && continue
      out=$(grep -nF "$pat" "$f" 2>/dev/null) || continue
      files=$((files+1))
      c=$(printf '%s\n' "$out" | wc -l)
      hits=$((hits+c))
      printf 'MATCH %s | %s | %s\n' "$pat" "$r/$f" "$(printf '%s\n' "$out" | head -5 | tr '\n' ';')" | tee -a "$L"
    done < <(git ls-files)
    echo "RESULT $r pattern='$pat' files=$files matching_lines=$hits" | tee -a "$L"
  done
done
