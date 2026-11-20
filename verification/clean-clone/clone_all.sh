# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
LOGDIR="$BASE/logs"
mkdir -p "$LOGDIR"
cd "$BASE" || exit 99

clone_one() {
  name="$1"
  url="$2"
  echo "=== CLONE: $name ==="
  echo "COMMAND: git clone $url $name"
  if [ -d "$name" ]; then
    echo "PRE-EXISTING DIRECTORY: $name (removing for a truly fresh clone)"
    rm -rf "$name"
  fi
  git -c credential.helper= clone --no-local "$url" "$name" 2>&1
  rc=$?
  echo "EXIT_CODE: $rc"
  if [ "$rc" -eq 0 ]; then
    echo "HEAD: $(git -C "$name" rev-parse HEAD 2>&1)"
    echo "REMOTE: $(git -C "$name" remote get-url origin 2>&1)"
  fi
  echo "=== END CLONE: $name ==="
  echo
}

{
  echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "git: $(git --version)"
  echo "note: plain https, no credentials, credential.helper disabled"
} | tee "$LOGDIR/clone-metadata.txt"

clone_one "proclaim-verify" "https://github.com/goldennftplatform-svg/proclaim-verify.git" 2>&1 | tee "$LOGDIR/clone-proclaim-verify.log"
clone_one "proclaim-labs" "https://github.com/goldennftplatform-svg/proclaim-labs.git" 2>&1 | tee "$LOGDIR/clone-proclaim-labs.log"
clone_one "proclaim-module-starter" "https://github.com/goldennftplatform-svg/proclaim-module-starter.git" 2>&1 | tee "$LOGDIR/clone-proclaim-module-starter.log"
