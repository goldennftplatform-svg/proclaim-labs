#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Local readiness check: tool health plus the real-world blockers.
set -euo pipefail
cd "$(dirname "$0")/.."

echo '=== tool checks ==='
bash tests/self-test.sh

echo
echo '=== readiness (honest, 2026-11-20) ==='
printf '%-42s %s\n' 'Testnet 0.4 endpoint' 'BLOCKED - no stable release (target 30 Nov 2026)'
printf '%-42s %s\n' 'Green E2E on default branch' 'IN FLIGHT - run 44695605569'
printf '%-42s %s\n' 'On-chain verifier program' 'NOT WRITTEN - 0.4 IDL unavailable'
printf '%-42s %s\n' 'Basecamp/QML GUI' 'NOT WRITTEN - tool name unresolved'
printf '%-42s %s\n' 'Third-party modules / users' '0 / 0 - cannot be manufactured'
printf '%-42s %s\n' 'Sustained activity / testimonials' '0 months / 0 - clock not started'
printf '%-42s %s\n' 'Deploy / testnet submission / solution PR' 'HELD - explicit approval required'
echo
echo 'Tools being ready is not the same as the underlying work being done.'
