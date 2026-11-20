# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
source ~/.cargo/env
echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "os_release: $(. /etc/os-release && echo "$PRETTY_NAME")"
echo "kernel: $(uname -r)"
echo "arch: $(uname -m)"
echo "bash: $BASH_VERSION"
echo "git: $(git --version)"
echo "rustc: $(rustc --version)"
echo "cargo: $(cargo --version)"
echo "rustfmt: $(cargo fmt --version)"
echo "clippy: $(cargo clippy --version)"
echo "python3: $(python3 --version 2>&1)"
echo "docker: $(command -v docker 2>/dev/null || echo 'not installed / not used')"
echo "lgs: $(command -v lgs 2>/dev/null || echo 'not installed')"
echo "CARGO_BUILD_JOBS: ${CARGO_BUILD_JOBS:-unset}"
echo "cargo registry cached .crate files: $(find "$HOME/.cargo/registry/cache" -type f -name '*.crate' 2>/dev/null | wc -l)"
