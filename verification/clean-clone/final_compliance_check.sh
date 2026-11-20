# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
cd "$BASE" || exit 99
echo "=== authored .md/.sh files in scope root and their first line ==="
for f in *.md *.sh; do
  [ -e "$f" ] || continue
  printf '%s | %s\n' "$f" "$(head -1 "$f")"
done
echo "=== target dirs remaining in proclaim-labs ==="
find proclaim-labs -type d -name target | wc -l
echo "=== clone HEADs ==="
for r in proclaim-verify proclaim-labs proclaim-module-starter; do echo "$r $(git -C "$r" rev-parse HEAD)"; done
echo "=== git status of clones (must be clean apart from untracked target removal) ==="
for r in proclaim-verify proclaim-labs proclaim-module-starter; do echo "--- $r ---"; git -C "$r" status --porcelain | head -10; done
echo "=== logs present ==="
ls -1 logs | sort
