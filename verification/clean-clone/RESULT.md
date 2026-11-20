<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# Clean-clone verification — RESULT

## Overall result

Three public repositories were cloned fresh over plain HTTPS and inspected on
WSL2 Ubuntu with `source ~/.cargo/env` and `CARGO_BUILD_JOBS=2`. The two
proclaim-verify test commands passed. Eight proclaim-labs workspaces were
checked with fmt / clippy / offline tests; see the per-workspace table below.
The module starter has no self-contained documented verification command; only
its JS example syntax was checked, and its README references a `RESULT.md` file
that is absent from the clone. Secret and SPDX scans found no `ghp_`,
`github_pat_` or uppercase `PRIVATE KEY` strings, and all counted authored
source files carry the SPDX identifier.

**This document reports reproduction results only. No criterion is claimed
complete. No adoption, chain, testnet, prize or deployment claim is made.**

## Pass/fail table

| Repo / workspace | Command(s) | Exit code(s) | Tests | Verdict |
| --- | --- | --- | --- | --- |
| proclaim-verify | `bash run-tests.sh` | 0 | 6 self-test checks + SPDX 7/7 | PASS |
| proclaim-verify | `bash tests/self-test.sh` | 0 | 6 checks | PASS |
| proclaim-labs / lp0001-phase0 | fmt + clippy + test --offline | 0 / 0 / 0 | 44 passed, 0 failed | PASS |
| proclaim-labs / lp0021-permissions | fmt + clippy + test --offline | 0 / 0 / 0 | 23 passed, 0 failed | PASS |
| proclaim-labs / lp0021-phase0 | fmt + clippy + test --offline | 0 / 0 / 0 | 23 passed, 0 failed | PASS |
| proclaim-labs / lp0021-phase1 | fmt + clippy + test --offline | 0 / 0 / 0 | 51 passed, 0 failed | PASS |
| proclaim-labs / lp0021-phase2-testimonials | fmt + clippy + test --offline | 0 / 0 / 0 | 50 passed, 0 failed | PASS |
| proclaim-labs / lp0021-phase3 | fmt + clippy + test --offline | 0 / 0 / 0 | 50 passed, 0 failed | PASS |
| proclaim-labs / lp0021-phase4-provider-protocol | fmt + clippy + test --offline | 0 / 0 / 0 | 45 passed, 0 failed | PASS |
| proclaim-labs / lp0023-claim-binding | fmt + clippy + test --offline | 0 / 0 / 0 | 40 passed, 0 failed | PASS |
| proclaim-labs TOTAL | — | — | 326 passed, 0 failed | PASS |
| proclaim-module-starter | documented workflow | not run (requires upstream repo, registry-api and lgs toolchain) | n/a | NOT VERIFIED |
| proclaim-module-starter | `node --check clients/js/search.mjs` | 0 | n/a | PASS (syntax only) |
| proclaim-module-starter | `cargo metadata --manifest-path Cargo.toml.template --no-deps` | 101 | n/a | EXPECTED FAILURE (template, not a manifest) |

## Exact commands

```
# clones (plain HTTPS, credentials disabled)
git -c credential.helper= clone --no-local https://github.com/goldennftplatform-svg/proclaim-verify.git proclaim-verify
git -c credential.helper= clone --no-local https://github.com/goldennftplatform-svg/proclaim-labs.git proclaim-labs
git -c credential.helper= clone --no-local https://github.com/goldennftplatform-svg/proclaim-module-starter.git proclaim-module-starter

# proclaim-verify
bash run-tests.sh
bash tests/self-test.sh

# each proclaim-labs workspace (source ~/.cargo/env; export CARGO_BUILD_JOBS=2)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --offline

# proclaim-module-starter
node --check clients/js/search.mjs
cargo metadata --manifest-path Cargo.toml.template --no-deps

# scan
git ls-files | xargs grep -nF 'ghp_'
git ls-files | xargs grep -nF 'github_pat_'
git ls-files | xargs grep -nF 'PRIVATE KEY'
git ls-files | xargs grep -nF 'private key'
grep -F 'SPDX-License-Identifier: MIT AND Apache-2.0' on each authored source file
```

## Evidence files (all inside `C:\Users\XPS\Downloads\GFD\staged\clean-clone-verification`)

- `FINDINGS.md` — full narrative, exact outputs, environment, limitations.
- `logs/clone-*.log`, `logs/clone-metadata.txt` — clone output and exit codes.
- `logs/verify-tests.log` — proclaim-verify exact output and exit codes.
- `logs/verify-break-checks.log` — extra documented break-attempt checks.
- `logs/labs-<workspace>.log`, `logs/labs-summary.txt`,
  `logs/labs-cleanup.log`, `logs/labs-run-console.log` — labs results.
- `logs/starter-checks*.log`, `logs/starter-lgs-check.txt` — module-starter.
- `logs/scan-all.log`, `logs/scan-secrets-corrected.log`,
  `logs/scan-summary.txt`, `logs/authored-*.txt` — secret/SPDX scan.
- `logs/env-details.txt` — measured environment.
- `*.sh` — the exact scripts used for the run (all SPDX-headed).

## Failed / could not verify

- proclaim-labs documentation discrepancy: root README says 323 total tests and
  20 for `lp0021-phase0`; the workspace README says 23 for that workspace and
  the fresh clone measured 23 (326 total). This is a doc inconsistency, not a
  test failure; no success beyond the measured numbers is claimed.
- proclaim-module-starter: no self-contained verification command exists;
  README's documented workflow needs an upstream checkout, a running API and
  the Logos toolchain. Not run.
- proclaim-module-starter: `README.md` references `RESULT.md`, which is absent
  from the fresh clone.
- `cargo metadata` on `Cargo.toml.template` fails with exit 101 by design
  (it is a template, not a real manifest).
- Any workspace that cannot build offline from the fresh clone is listed as a
  failure in the table above with its exact error; no success is claimed for
  such a workspace.
- Cold/empty Cargo cache behaviour, network-fresh dependency resolution, real
  guest builds, proof generation, chain/testnet/deployment and prize criteria
  were not tested and are not claimed.
