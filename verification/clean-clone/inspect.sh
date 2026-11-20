# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
LOGDIR="$BASE/logs"
mkdir -p "$LOGDIR"
cd "$BASE" || exit 99

for r in proclaim-verify proclaim-labs proclaim-module-starter; do
  echo "################ REPO: $r ################"
  echo "--- git metadata ---"
  echo "HEAD: $(git -C "$r" rev-parse HEAD 2>&1)"
  echo "HEAD_DATE: $(git -C "$r" log -1 --format='%cI %s' 2>&1)"
  echo "--- top-level listing ---"
  ls -la "$r" 2>&1
  echo "--- tracked files (all) ---"
  git -C "$r" ls-files 2>&1
  echo
done
