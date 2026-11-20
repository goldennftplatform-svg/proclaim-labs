# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
L="$BASE/logs/verify-break-checks.log"
cd "$BASE/proclaim-verify" || exit 99
{
echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
run() {
  echo "############ COMMAND: $* ############"
  "$@" 2>&1
  echo "EXIT_CODE: $?"
  echo
}
run ./bin/verify-source base58 4df7d5396b68a16aae6d0f768ef447a0055eb43866827154f71139e486265144
run ./bin/verify-source base58 ABCD
run ./bin/verify-source base58 zzzz
echo "############ self-test from different cwd ############"
cd /tmp || exit 99
bash "$BASE/proclaim-verify/tests/self-test.sh" 2>&1
echo "EXIT_CODE: $?"
} 2>&1 | tee "$L"
