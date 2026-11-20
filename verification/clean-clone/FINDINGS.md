<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# Clean-clone verification — FINDINGS

## Scope and honesty statement

This is an independent, read-only verification of three **public** GitHub
repositories from **fresh clones**. It was performed only inside
`C:\Users\XPS\Downloads\GFD\staged\clean-clone-verification`. No existing staged
directory, git repository, public remote, credential, deployment, Basecamp
instance or prize repository was modified. No pushes, PRs, issues, Docker,
tokens or keys were used.

Plain HTTPS URLs were used for cloning with credential helpers explicitly
disabled (`git -c credential.helper= clone --no-local <https-url> <dir>`).

**No criterion is claimed complete by this document.** These are local
reproduction results only. See the "Not tested / not claimed" section at the
end and in `RESULT.md`.

## Environment (measured, not assumed)

| Item | Observed value |
| --- | --- |
| Host | Windows, WSL2 |
| WSL distro | Ubuntu 26.04.1 LTS |
| Kernel | `6.18.58.1-microsoft-standard-WSL2`, x86_64 |
| Shell | GNU bash 5.3.9(1)-release |
| git | `git version 2.53.0` |
| Rust | `rustc 1.100.0 (0ef1bc386 2026-11-09)` |
| Cargo | `cargo 1.100.0 (3acc79c0c 2026-11-05)` |
| rustfmt | `rustfmt 1.11.0-stable (0ef1bc3860 2026-11-09)` |
| clippy | `clippy 0.1.100 (0ef1bc3860 2026-11-09)` |
| python3 | `Python 3.14.4` |
| `lgs` | `/home/xps/.cargo/bin/lgs`, `logos-scaffold 0.5.0` (present after `source ~/.cargo/env`) |
| `CARGO_BUILD_JOBS` | `2` (exported for all cargo commands in this run) |
| Cargo setup | `source ~/.cargo/env` required before cargo commands |
| Cargo registry cache | 1128 cached `.crate` files in `~/.cargo/registry/cache` (warm cache; see caveat below) |
| Docker | Not installed and not used |
| Node | Windows `node.exe` at `/mnt/c/Program Files/nodejs/node.exe` was used for one syntax check; `node` is not on the WSL PATH |

Caveat: the Cargo registry cache on this machine was already warm. `cargo test
--workspace --offline` results therefore prove only that these clones build
**with this local crate cache**. A machine with an empty cargo cache and no
network access is **not** covered by this report. The two workspaces with
crates.io dependencies (`lp0001-phase0`, `lp0021-phase0`) had all their required
external crate versions already cached (`logs/offline-cache-check2.txt`); the
six other workspaces are dependency-free. Clippy for `lp0021-phase0` fetched no
new crates and reached the `libsodium-sys` local C build.

## Task A — fresh clones

All three clones succeeded (exit code 0) from plain HTTPS URLs.

| Repo | URL | HEAD | HEAD subject |
| --- | --- | --- | --- |
| proclaim-verify | `https://github.com/goldennftplatform-svg/proclaim-verify.git` | `310c85010a82a0df9afe6f247e47199338e55172` | `Docs: add public battle-test kit` |
| proclaim-labs | `https://github.com/goldennftplatform-svg/proclaim-labs.git` | `b5e5d8dc4f837e649a56aa55dfbcaa9cf2a1d8ee` | `Docs: publish honest LP-0023 self-assessment` |
| proclaim-module-starter | `https://github.com/goldennftplatform-svg/proclaim-module-starter.git` | `66a12a3462b2b106aa703bc212bd3abd71592a2e` | `Initial LP-0023 registry module starter template` |

Exact command form (one per repo, in `clone_all.sh`):

```
git -c credential.helper= clone --no-local https://github.com/goldennftplatform-svg/proclaim-verify.git proclaim-verify
git -c credential.helper= clone --no-local https://github.com/goldennftplatform-svg/proclaim-labs.git proclaim-labs
git -c credential.helper= clone --no-local https://github.com/goldennftplatform-svg/proclaim-module-starter.git proclaim-module-starter
```

Recorded output and exit codes: `logs/clone-proclaim-verify.log`,
`logs/clone-proclaim-labs.log`, `logs/clone-proclaim-module-starter.log`,
`logs/clone-metadata.txt`. Each clone printed `Cloning into '<name>'...` and
`EXIT_CODE: 0`. proclaim-labs additionally printed the normal
`Updating files: ... 100% (121/121), done.` progress lines.

## Task B — proclaim-verify

Working directory: `clean-clone-verification/proclaim-verify`.

Command and exact output:

```
$ bash run-tests.sh
=== shell syntax ===
syntax OK
=== self-test ===
PASS: base58 known vector
PASS: base58 all-zero vector
PASS: reject short hex
PASS: reject non-hex
PASS: reject missing binary
PASS: usage exit code
self-test: all checks passed
=== SPDX headers ===
SPDX: 7/7
all checks passed
EXIT_CODE: 0

$ bash tests/self-test.sh
PASS: base58 known vector
PASS: base58 all-zero vector
PASS: reject short hex
PASS: reject non-hex
PASS: reject missing binary
PASS: usage exit code
self-test: all checks passed
EXIT_CODE: 0
```

Result: **PASS / PASS** (exit 0 for both). Six self-test assertions passed on
each run. Full log: `logs/verify-tests.log`.

Additional documented "try to break" checks from `BATTLE-TEST.md` (exact
output in `logs/verify-break-checks.log`):

```
$ ./bin/verify-source base58 4df7d5396b68a16aae6d0f768ef447a0055eb43866827154f71139e486265144
6FMaVQ6CmbAVNHgwsmmeBe5kiiKkQRFtqu4GUQ2vAf2T
EXIT_CODE: 0

$ ./bin/verify-source base58 ABCD
ERROR: expected 64 lowercase hex characters
EXIT_CODE: 1

$ ./bin/verify-source base58 zzzz
ERROR: expected 64 lowercase hex characters
EXIT_CODE: 1

$ bash <scope>/proclaim-verify/tests/self-test.sh   # run from /tmp
PASS: base58 known vector
PASS: base58 all-zero vector
PASS: reject short hex
PASS: reject non-hex
PASS: reject missing binary
PASS: usage exit code
self-test: all checks passed
EXIT_CODE: 0
```

The expected base58 vector from the README matched exactly.

## Task C — proclaim-labs (8 Rust workspaces)

`README.md` identifies these eight workspaces:
`lp0001-phase0/`, `lp0021-permissions/`, `lp0021-phase0/`, `lp0021-phase1/`,
`lp0021-phase2-testimonials/`, `lp0021-phase3/`,
`lp0021-phase4-provider-protocol/`, `lp0023-claim-binding/`.

For each workspace, from inside that workspace and with
`source ~/.cargo/env; export CARGO_BUILD_JOBS=2`, these exact commands were run:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --offline
```

### Measured results (all 8 passed)

| Workspace | `fmt` exit | `clippy` exit | `test --offline` exit | Tests passed | Failed | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| `lp0001-phase0` | 0 | 0 | 0 | 44 | 0 | PASS |
| `lp0021-permissions` | 0 | 0 | 0 | 23 | 0 | PASS |
| `lp0021-phase0` | 0 | 0 | 0 | 23 | 0 | PASS |
| `lp0021-phase1` | 0 | 0 | 0 | 51 | 0 | PASS |
| `lp0021-phase2-testimonials` | 0 | 0 | 0 | 50 | 0 | PASS |
| `lp0021-phase3` | 0 | 0 | 0 | 50 | 0 | PASS |
| `lp0021-phase4-provider-protocol` | 0 | 0 | 0 | 45 | 0 | PASS |
| `lp0023-claim-binding` | 0 | 0 | 0 | 40 | 0 | PASS |
| **Total** | — | — | — | **326** | **0** | **PASS** |

`cargo fmt --all -- --check` produced no diff in any workspace.
`cargo clippy --workspace --all-targets -- -D warnings` exited 0 with no
`warning:` lines in any workspace log (clippy ran in 10m47s for
`lp0021-phase0` because `libsodium-sys` builds and checks libsodium's C test
suite from source; the other seven workspaces were fast).
`cargo test --workspace --offline` exited 0 everywhere; every `test result:`
line reported `ok` with 0 failures.

Raw per-workspace logs: `logs/labs-<workspace>.log`. Aggregate summary:
`logs/labs-summary.txt`. Runner: `run_labs_checks.sh`. Console transcript:
`logs/labs-run-console.log`.

Discrepancy found in the repository's own documentation (not a test failure):
the root README table lists **323** total tests and **20** for
`lp0021-phase0`, while that workspace's own README says **23 tests passed**
(wallet-core 12, wallet-keystore 6, wallet-types 5). The fresh clone measured
**23** for `lp0021-phase0` and **326** total. The other seven workspace counts
match their READMEs exactly (44, 23, 51, 50, 50, 45, 40). No claim is made here
about ZK, chain, adoption or prize criteria.

Cleanup: all eight `target/` directories were removed at the end of the run;
`remaining target dirs: 0`. The exact cleanup transcript is
`logs/labs-cleanup.log`.

## Task D — proclaim-module-starter

The README does **not** specify a self-contained verification command. It is a
starter template: there is no real `Cargo.toml`, no `package.json`, no test
script and no `Makefile` in the clone. Every command documented in the README
and `docs/INTEGRATION-CHECKLIST.md` requires external prerequisites that are
not part of this fresh clone:

- `npm install ./clients/js` — requires a checkout of the *upstream* `proclaim`
  repo (not present);
- `node search.mjs` — requires a running `registry-api` on `127.0.0.1:3030`
  (not started; out of scope);
- `cargo run --example register_and_search` — requires copying
  `Cargo.toml.template` to `Cargo.toml` and filling pinned upstream git deps;
- `lgs` / Basecamp toolchain / a disposable sequencer — outside the scope of
  this verification (no chain, no deployment).

Executable checks that were run:

```
$ cargo metadata --manifest-path Cargo.toml.template --no-deps
error: the manifest-path must be a path to a Cargo.toml file: `.../Cargo.toml.template`
EXIT_CODE: 101
```
(expected for a template file, not a buildable manifest)

```
$ "/mnt/c/Program Files/nodejs/node.exe" --check clients/js/search.mjs
EXIT_CODE: 0
```

Finding: `README.md` line 107 lists `RESULT.md`, but **`RESULT.md` is absent**
from the fresh clone (confirmed by `git ls-files` and filesystem check).
`logs/starter-checks.log`, `logs/starter-checks2.log` and
`logs/starter-lgs-check.txt` contain the full record. An early run of
`starter-checks.log` reported `lgs: MISSING` and `cargo: MISSING` because
`~/.cargo/env` had not been sourced; `starter-checks2.log` and
`starter-lgs-check.txt` correct this (`lgs = logos-scaffold 0.5.0`).

**This repository is NOT verified as a working module**: only the JS example's
syntax was checked. No install, no network client run, no `.lgx` build was
performed.

## Task E — secret and SPDX scan

Method: `git ls-files` (tracked files only, `.git` excluded), literal
`grep -nF`. Patterns: `ghp_`, `github_pat_`, `PRIVATE KEY` (uppercase PEM-style),
and additionally lowercase `private key` for completeness.

"Authored source files" for the SPDX count = tracked text files with extensions
`.rs .toml .sh .md .mjs .js .py .qml .cpp .h .cmake`, files named
`CMakeLists.txt`, plus extensionless scripts under `bin/` and `tests/`.
Excluded: `Cargo.lock` (generated), `LICENSE-MIT` / `LICENSE-APACHE`
(standard legal texts), all `.git` content. The exact search string counted is
`SPDX-License-Identifier: MIT AND Apache-2.0`.

| Repo | Tracked files | Authored source files | With SPDX | `ghp_` | `github_pat_` | `PRIVATE KEY` | lowercase `private key` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| proclaim-verify | 10 | 8 | 8/8 | 0 | 0 | 0 | 0 |
| proclaim-labs | 121 | 111 | 111/111 | 0 | 0 | 0 | 1 file / 4 lines |
| proclaim-module-starter | 13 | 9 | 9/9 | 0 | 0 | 0 | 1 file / 3 lines |

The lowercase `private key` hits are **documentation strings, not secrets**:

- `proclaim-labs/lp0021-permissions/consent-store/src/lib.rs:125,131,141,150`
  — string literals `"your private keys"` used as test/demo descriptions.
- `proclaim-module-starter/docs/INTEGRATION-CHECKLIST.md:65,152,193` —
  warnings such as "Do not copy any private key into your repository" and
  "Never publish private keys, tokens or capability tokens in a repo".

No `ghp_`, no `github_pat_`, and no uppercase `PRIVATE KEY` was found in any
tracked file of any of the three clones. This is a literal string scan only; it
is not a claim that no credential of any form exists anywhere.

Full logs: `logs/scan-all.log`, `logs/scan-secrets-corrected.log`,
`logs/scan-summary.txt`, and `logs/authored-<repo>.txt` file lists. Note: the
`grep_rc=` numbers printed in `logs/scan-all.log` came from `git ls-files` in a
pipeline (always 0) and are not meaningful; use the file/line counts from
`logs/scan-secrets-corrected.log` instead.

## Not tested / not claimed

- No on-chain transaction, chain, sequencer, LEZ, testnet or official-zone work.
- No prize criterion is claimed complete; no LP-0001 / LP-0021 / LP-0023
  criterion credit is claimed.
- No deployment, no installation into Basecamp, no `.lgx` package build.
- No solution PR, issue, git push or public remote write.
- No adoption, users, testimonials, verification counts or third-party module
  claims.
- No real RISC Zero guest build, `repro`, proof generation or proof
  verification; `lgs program-id`, `check`, `repro` and `real-binary-smoke.sh`
  require a real built guest and were **not** run.
- No Docker (not installed).
- No claim about a machine with a cold/empty Cargo registry cache.
- proclaim-module-starter's documented integration workflow (sequencer,
  register, `make api`, JS client, Rust example) was **not** run.
