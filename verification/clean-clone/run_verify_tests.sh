# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
LOGDIR="$BASE/logs"
mkdir -p "$LOGDIR"
cd "$BASE/proclaim-verify" || exit 99

echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "cwd: $(pwd)"
echo "HEAD: $(git rev-parse HEAD)"
echo "python3: $(python3 --version 2>&1)"

echo "############ COMMAND: bash run-tests.sh ############"
bash run-tests.sh 2>&1
rc1=$?
echo "############ EXIT_CODE: $rc1 ############"
echo

echo "############ COMMAND: bash tests/self-test.sh ############"
bash tests/self-test.sh 2>&1
rc2=$?
echo "############ EXIT_CODE: $rc2 ############"
echo

echo "SUMMARY run-tests.sh=$rc1 self-test.sh=$rc2"
