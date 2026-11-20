#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Remove only this workspace's build directory.
set -euo pipefail
cd "$(dirname "$0")"
rm -rf ./target
