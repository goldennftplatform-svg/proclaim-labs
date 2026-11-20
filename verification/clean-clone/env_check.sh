# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
echo "=== ENV CHECK ==="
echo "date: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "uname: $(uname -a)"
echo "shell: $BASH_VERSION"
echo "user: $(whoami)"
echo "home: $HOME"
echo "pwd: $(pwd)"
for t in git cargo rustc rustup bash curl grep find; do
  p=$(command -v "$t" 2>/dev/null) || p=MISSING
  echo "tool $t: $p"
done
echo "--- versions ---"
git --version 2>&1 || true
rustc --version 2>&1 || true
cargo --version 2>&1 || true
echo "--- cargo env file ---"
ls -la "$HOME/.cargo/env" 2>&1 || true
echo "--- scope dir ---"
ls -la /mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification 2>&1 || true
echo "=== END ENV CHECK ==="
