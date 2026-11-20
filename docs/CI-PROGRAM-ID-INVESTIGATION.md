<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# FINDINGS — guest image-ID mismatch in proclaim E2E run 44678946709

**Scope:** read-only diagnosis. No repository was modified, no git operation was
performed, no credentials were used. Public source was fetched by HTTPS into
this directory; local WSL evidence was read only; reproduction builds were made
only in isolated copies under this directory.

## 0. Bottom line

The build inputs (guest source, both Cargo.lock files, git dependency
revisions, `RUSTFLAGS`, make-bin source/version, and guest toolchain) are
**identical** between the prior successful local build and commit `29da69c`.
A clean rebuild from the *same* inputs but from a **different absolute source
path** produced a **different image ID** (`daf7b2cdd42bf6e07e42850f0b5fcdd631e8c16855a60d3f4879ec47a3235f2d`),
even though the guest toolchain and `RUSTFLAGS` matched. Byte-level analysis
found exactly **one differing byte inside the loaded ELF image**: the low byte
of the ELF header's `e_shoff` field (`0x24` local vs `0x00` rebuild). That field
shifts because Cargo's path-derived crate-disambiguator changes symbol-string
lengths in `.strtab`. GitHub Actions checks out to `/home/runner/work/proclaim/proclaim`,
whereas the successful local build was made from
`/home/xps/logos-work/registry`. Therefore the **most likely cause is that the
image ID is build-path dependent**: the recorded `4df7d539…65144` is the ID of
an artifact built at the local path, not of an artifact built at the Actions
checkout path. This is a hypothesis, not a proof of CI's exact `5f80583a…d733`;
the proving command is given in §4.

The E2E criterion is **not met**. This document does not claim otherwise.

## 1. Exact evidence

### 1.1 Fetched public source and CI metadata

```
curl.exe -L --fail -o proclaim-29da69c.tar.gz \
  https://codeload.github.com/goldennftplatform-svg/proclaim/tar.gz/29da69c
tar.exe -xzf proclaim-29da69c.tar.gz
```

- Tarball kept: `proclaim-29da69c.tar.gz` (260,983 bytes).
- Fetched `.github/workflows/ci.yml` SHA-256
  `168C130C03BE0EBD0B34E98B70694351BA004B94442AAD18C94D08E8F9B25230`
  is byte-identical to `staged/e2e-ci/ci.yml` (lines 160–178 are the failing step).
- `GET /repos/goldennftplatform-svg/proclaim/actions/runs/44678946709`:
  `head_sha=29da69c90bdb7e492010e3483f9454b4f299f364`,
  `run_number=10`, `conclusion=failure`, `created_at=2026-11-20T05:07:50Z`.
- Jobs: `rust` success, `js` success, `e2e` failure at step 12
  "Build guest locally and assert deployed program ID"; step 13 skipped.
- Commit `29da69c` changed **only** `.github/workflows/ci.yml` (3 added lines:
  `mkdir -p …/docker`). It did not change guest source.
- Earlier commits: `955c4c1` changed `rzup install rust 1.94.1` → `1.97.0`;
  `33e635d` restored step-level `RUSTFLAGS: '--cfg getrandom_backend="custom"'`
  on the guest-build step. Both are present at `29da69c`.

### 1.2 Toolchain / lock / pin inspection of fetched tree

- No `rust-toolchain.toml`, no `.cargo/config` or `.cargo/config.toml` anywhere
  in the repository. Guest toolchain is chosen entirely by
  `rzup install rust 1.97.0` in the workflow.
- `methods/guest/Cargo.toml` uses **tags**, not revs:
  `spel-framework tag v0.7.0`, `lee_core/nssa_core tag v0.2.4`,
  `risc0-zkvm = "=3.0.5"`.
- `methods/guest/Cargo.lock` pins the exact revs:
  `git+https://github.com/logos-co/spel.git?tag=v0.7.0#4107dcb14ec0eb64e9e80a9691a3df0b9acd3c2b`
  and
  `git+https://github.com/logos-blockchain/logos-execution-zone.git?tag=v0.2.4#47eba256479f6f785acbd138834340703cd03401`.
  These match `scaffold.toml` pins exactly.
- Public GitHub tag lookups today (read-only API):
  `spel v0.7.0 -> 4107dcb14ec0eb64e9e80a9691a3df0b9acd3c2b`,
  `LEZ v0.2.4 -> 47eba256479f6f785acbd138834340703cd03401`
  — **no tag drift**.
- `tools/make-bin/src/main.rs` in the repo is byte-identical (SHA-256
  `92729DE2DFC429DA6788BCCA82693C876E3C174C685F681AAF19311D80692393`) to
  `staged/e2e-ci/make-bin/src/main.rs`. It uses
  `risc0-binfmt = "=3.0.5"` and `risc0-zkos-v1compat = "2.2.3"`.
- crates.io: latest stable `risc0-zkos-v1compat` is `2.2.3` (published
  2026-07-17), so a fresh CI resolution cannot silently pick a newer stable
  wrapper ELF. `risc0-binfmt` is exactly pinned.

### 1.3 Prior local evidence (read-only)

Found under WSL `/home/xps` (never edited):

| Item | Value |
|---|---|
| local guest ELF | `/home/xps/logos-work/registry/methods/guest/target/riscv32im-risc0-zkvm-elf/release/registry` |
| local ELF SHA-256 | `72cdfd33128068adb2512ffb3f7c7986e9503b688e878a9df53566f57fef5fba` |
| local R0BF `.bin` | `…/docker/registry.bin`, size 463,236 |
| local `.bin` SHA-256 | `88716a6ba8c67e9623f3b2d5acf1d9c758ce071dbce391f1d9084628d72506c5` |
| `lgs spel -- program-id <local bin> --format hex` | `4df7d5396b68a16aae6d0f768ef447a0055eb43866827154f71139e486265144` |

Note: staged docs call `88716a6b…` the "ELF SHA-256"; it is in fact the SHA-256
of the **R0BF-wrapped `.bin`** (the raw ELF hash is `72cdfd33…`). This is a
documentation mislabel, not a cause of the failure.

The recorded `.lgx` root hash `4979d0f4992d9678cb5b07efc9d12b22938a59bbed0f42f471d5cc477fd53f50`
(`staged/LGX-BUILD.md:88`) is a UI module manifest hash and is unrelated to the
guest image ID. A hash beginning `102403…` was **not** found in the staged docs
(the only `102403…` match is an unrelated numeric token inside a Cargo
fingerprint JSON); no claim is made about it.

Recursive read-only diff of the entire fetched `29da69c` tree against the local
source tree (excluding `target/`, `.git/`, and generated dirs) showed **no
differing source files**. Specifically identical by SHA-256:

- `methods/guest/Cargo.toml`
- `methods/guest/Cargo.lock` → `5c7886e17fe66c4944bd1b293a19915e453fd707e7c2e551996e9b5c5154c1bc`
- `methods/guest/src/bin/registry.rs` → `b7847ad11ffaa7361c4105d23c2a4fbba11f810b816356578d6a3fdf3206326f`
- `registry_core/src/lib.rs` → `4ab26940f664e8f04e9524bf10043b64cd822c61173ac7e8bb97fd23cf2f60d3`
- root `Cargo.lock` → `eb76f2966c8a4a5b17a2c86347f7a7046153b55f39460d972ca4e44802ef3cfe`

Local cargo git checkouts exist only at the pinned revs
(`spel-411f019390120158/4107dcb`,
`logos-execution-zone-6bae42d7c9cadfe7/47eba25`). No `.cargo/config*` file
exists locally or in the repo. Local guest toolchain: only
`~/.risc0/toolchains/v1.97.0-rust-x86_64-unknown-linux-gnu`,
`cargo +risc0 --version` = `cargo 1.97.0-dev (c980f4866 2026-06-30)`,
`~/.risc0/settings.toml` `rust = "1.97.0"`.

### 1.4 Controlled reproduction inside this directory

Commands (scripts kept here: `setup-repro.sh`, `repro-build.sh`):

```
cp -a proclaim-29da69c repro-ci-flags
cd repro-ci-flags
export CARGO_BUILD_JOBS=2
export RUSTFLAGS='--cfg getrandom_backend="custom"'
cargo +risc0 build --release --manifest-path methods/guest/Cargo.toml \
  --target riscv32im-risc0-zkvm-elf
cargo build --release --manifest-path tools/make-bin/Cargo.toml
mkdir -p methods/guest/target/riscv32im-risc0-zkvm-elf/docker
tools/make-bin/target/release/make-bin \
  methods/guest/target/riscv32im-risc0-zkvm-elf/release/registry \
  methods/guest/target/riscv32im-risc0-zkvm-elf/docker/registry.bin
lgs spel -- program-id …/docker/registry.bin --format hex
```

Observed (exact):

| Build | raw ELF SHA-256 | `.bin` SHA-256 | image ID |
|---|---|---|---|
| prior local `/home/xps/logos-work/registry` | `72cdfd33…5fba` | `88716a6b…06c5` | `4df7d539…65144` |
| isolated rebuild with CI `RUSTFLAGS`, path `/mnt/c/…/repro-ci-flags` | `3564a6743efdb0d30584ce20b79a2688ede53f2781dc4246da8f8753fca33cbb` | `5bf04cdf542d0f2cd8c1157f04e005ef18037a21cff7564c0eaa9619c5daf6b8` | `daf7b2cdd42bf6e07e42850f0b5fcdd631e8c16855a60d3f4879ec47a3235f2d` |
| CI run 44678946709, path `/home/runner/work/proclaim/proclaim` | not published | not published | `5f80583a57b91af90f5e30d3420379ca4e4f7abeecb2c975b6063cfcd944d733` |

A control build with `RUSTFLAGS` unset **failed to compile**:
`getrandom 0.3.4` emitted `target is not supported. You may need to define a
custom backend` for `riscv32im-risc0-zkvm-elf`. Therefore the custom-backend
cfg is genuinely required and was necessarily present in the successful local
build; a simple "CI forgot RUSTFLAGS" explanation is not viable.

The guest `Cargo.lock` was unchanged by the reproduction (same SHA-256 as
fetched). Dependency versions observed in the rebuild log match those embedded
as source paths in the prior local ELF (e.g. `anyhow-1.0.104`, `borsh-1.8.1`,
`risc0-zkvm-3.0.5`, `risc0-binfmt-3.0.6`).

### 1.5 What exactly differs in the ELF — byte-level causal chain

`readelf -lW` shows both ELFs have identical program headers and the same three
`PT_LOAD` regions:

| LOAD | offset / size | bytes LOCAL | bytes NEW |
|---|---|---|---|
| 1 | `0x000000 / 0x91c0` (ELF header + `.rodata` + `.eh_frame`) | `81692289851f5ddeacc41b415ad071f6e7d640c2f2dd74e97303802f45c3fddf` | `911e4c00b6ce95aa24d1d990efc73a69284f608a570bb9bc8b93b577d0990100` |
| 2 | `0x0091c0 / 0x35b74` (`.text`) | `83bf39439d6142509123e2b7f48c4b233885d71a27485f2b0a5145937a111b40` | identical |
| 3 | `0x03ed34 / 0x00058` (`.data`) | `ae68f34dfedef48336896371b9dde7491fa670fbb1b1e262d71e22267900a34f` | identical |

`cmp -l -n 0xf8` localises the LOAD-1 difference to **one byte at file offset
32 (1-based offset 33)**: the low byte of `e_shoff` is `0x24` locally and `0x00`
in the rebuild, i.e. `0x69124` vs `0x69100`. All other bytes in the loaded
region are identical, including `e_entry` (`0x2fcd4`), `.comment` (same rustc
version string), `.text`, `.rodata`, `.eh_frame` and `.data`.

The reason `e_shoff` moves is visible in `readelf -SW`: `.symtab` is identical
(0xecc0 bytes, 3559 symbols) and `.comment` / `.riscv.attributes` are identical,
but `.strtab` is `0x1b561` bytes locally vs `0x1b53f` in the rebuild (34 bytes
shorter), shifting the section-header table by 36 bytes. `strings` comparison
shows the symbol names differ in Cargo's crate-disambiguator hash, e.g. local
`…Csj5ewmAqrWAn_8registry` vs rebuild `…CsjWtNoIvxOfi_8registry`, with
corresponding `.Lanon.<hash>` labels. That disambiguator is derived from the
package source ID, which for a path package includes its absolute manifest
path.

This matters because `risc0-binfmt 3.0.6::Program::load_elf` copies every
`PT_LOAD` byte — including the ELF header — into the guest memory image;
`ProgramBinary::compute_image_id()` then hashes that image. So a metadata-only
change in `.strtab` becomes a real one-byte change in the hashed memory image.
This is direct evidence that **changing the checkout path changes the R0BF
bytes and therefore the image ID** with byte-identical source, flags and
compiler. It does not by itself prove that `/home/runner/work/proclaim/proclaim`
is the only difference in CI (see §2/§6).

Cross-wrap controls (`cross-wrap.sh`): the local and rebuilt `make-bin` binaries
are byte-identical (SHA-256 `ce4eb3a64401ee4a1a5660fdd263e554aeb563c9b3e1aaeba985a79f918e796d`),
and wrapping either ELF with either helper yields only the ELF-derived hash
(`88716a6b…` or `5bf04cdf…`), so the wrapper is not the variable.

## 2. Ranked hypotheses

| # | Hypothesis | Evidence for | Evidence against / status |
|---|---|---|---|
| 1 | **Build-path dependence of Cargo crate metadata** (local `/home/xps/logos-work/registry` vs Actions `/home/runner/work/proclaim/proclaim`; also our `/mnt/c/…` rebuild) | Same source, lock, toolchain and `RUSTFLAGS` produced a third distinct ID from a third path; byte diff isolates it to path-derived symbol strings → `.strtab` length → `e_shoff` → one loaded byte → image ID | Not yet reproduced at the exact Actions path; proving command in §4. **Top hypothesis.** |
| 2 | Guest toolchain patch/build drift (`rzup` 1.97.0 build used by CI may differ from the local 2026-06-30 build) | Both local and CI use "1.97.0"; rzup resolves a release, not a byte-pinned compiler; compiler metadata would change symbol hashes | Local toolchain is 1.97.0-dev c980f4866; CI log shows the install step succeeded but the exact compiler commit is not in the public metadata we read. Possible, not proven. |
| 3 | `RUSTFLAGS` difference between local and CI | Flag affects codegen; a missing custom backend fails to compile (shown) | Both local docs/`verify-source.sh` and CI step set the identical string; our rebuild with that string compiled. Low. |
| 4 | `Cargo.lock` regeneration / crate version drift | Guest build does not pass `--locked`; make-bin has no committed lock | Guest lock byte-identical locally and in repo; tags resolve to the locked revs; make-bin deps resolve to same versions (`binfmt 3.0.5`, `v1compat 2.2.3` latest stable). Ruled out for the reviewed inputs. |
| 5 | Git dependency pin drift (SPEL `4107dcb1`, LEZ `47eba256`) | Manifests use mutable tags | Lock pins exact revs and public tags still point there; local checkouts at same revs. Ruled out. |
| 6 | make-bin transformation difference | Wrapper ELF is embedded in the image ID | Repo and staged source byte-identical; local `.bin` hash `88716a6b…` reproduces the expected ID from the local ELF. Ruled out. |
| 7 | Source drift between deployed artifact and `29da69c` | Deployed ID predates some commits | Full recursive diff: no source differences; `29da69c` touched only CI YAML; licence-comment change in `d2e48f3` cannot affect codegen. Ruled out. |
| 8 | Wrong expected ID | — | Local artifact with the same source produces exactly `4df7d539…65144`; multiple docs agree. Ruled out as "wrong" — it is a *path-specific* ID. |

## 3. Single most likely cause

**The image ID is not portable across build directories.** Cargo embeds a
path-derived crate-disambiguator hash in symbol names; its length changes
`.strtab`, which moves `e_shoff` inside the ELF header, and that header is part
of the loaded image hashed by `spel program-id`. The deployed/expected ID was
produced at `/home/xps/logos-work/registry`; the clean CI checkout builds at
`/home/runner/work/proclaim/proclaim`. The mismatch is therefore most likely an
artefact of asserting a locally-derived ID in a different build path, not a
broken build, not a pin regression, and not a wrong source tree. The exact CI
ID remains unverified until §4 step 1 is run.

## 4. Exact commands to verify each hypothesis

All commands below are read-only with respect to any repository; run them in a
throwaway directory.

1. **Path dependence (top hypothesis).** Build the same source at the exact
   Actions path and compare with run 44678946709:
   ```bash
   sudo mkdir -p /home/runner/work/proclaim/proclaim
   sudo chown "$USER" /home/runner/work/proclaim /home/runner/work/proclaim/proclaim
   cp -a <fetched-29da69c-tree>/. /home/runner/work/proclaim/proclaim/
   cd /home/runner/work/proclaim/proclaim
   export RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_BUILD_JOBS=2
   cargo +risc0 build --release --manifest-path methods/guest/Cargo.toml \
     --target riscv32im-risc0-zkvm-elf
   cargo build --release --manifest-path tools/make-bin/Cargo.toml
   mkdir -p methods/guest/target/riscv32im-risc0-zkvm-elf/docker
   tools/make-bin/target/release/make-bin \
     methods/guest/target/riscv32im-risc0-zkvm-elf/release/registry \
     methods/guest/target/riscv32im-risc0-zkvm-elf/docker/registry.bin
   lgs spel -- program-id \
     methods/guest/target/riscv32im-risc0-zkvm-elf/docker/registry.bin --format hex
   # EXPECT 5f80583a57b91af90f5e30d3420379ca4e4f7abeecb2c975b6063cfcd944d733
   ```
   (This writes only to a scratch path; it was **not** run here because the
   scope forbids writing outside this investigation directory.)

2. **Toolchain drift.** In the successful CI job log, run
   `"$HOME/.risc0/toolchains/"*/bin/rustc -Vv` and compare the commit hash with
   the local `cargo 1.97.0-dev (c980f4866 2026-06-30)`. Then rebuild with
   `RUSTC_BOOTSTRAP` unchanged and that exact compiler.

3. **RUSTFLAGS.** Re-run the isolated build with
   `RUSTFLAGS='--cfg getrandom_backend="custom" -C metadata=probe'`; any ID
   change confirms flags are hashed. (Already established: unsetting the flag
   fails to compile for this target.)

4. **Lock regeneration.** `sha256sum methods/guest/Cargo.lock` before and after
   `cargo +risc0 build --locked`; add `--locked` to the CI step and re-run.

5. **Git pins.** Public API:
   `curl -s https://api.github.com/repos/logos-co/spel/git/ref/tags/v0.7.0`
   and `…/logos-execution-zone/git/ref/tags/v0.2.4` (already checked; both
   match). Also `grep -n '4107dcb1\|47eba256' methods/guest/Cargo.lock`.

6. **make-bin.** `sha256sum tools/make-bin/src/main.rs` in repo and staged;
   verify `risc0-zkos-v1compat` selected version with
   `cargo tree --manifest-path tools/make-bin/Cargo.toml -i risc0-zkos-v1compat`.

7. **Source drift.** `diff -rq --exclude=target --exclude=.git <fetched> <local>`;
   `curl -s …/commits/29da69c` via GitHub API to confirm only CI YAML changed.

8. **Expected ID.** Confirm locally:
   `lgs spel -- program-id /home/xps/logos-work/registry/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/registry.bin --format hex`
   (observed `4df7d539…65144`).

## 5. Recommended minimal fix (NOT applied)

Two acceptable minimal fixes; choose based on policy:

- **Preferred if source-verification must hold:** stop asserting a
  path-specific constant. Make the CI step derive the ID from the clean build,
  assert that two clean builds at the *same canonical path* produce the same
  ID, and feed that ID into `e2e-real-sequencer.sh` / the browser / API
  (e.g. export `REGISTRY_PROGRAM_ID` from the built `.bin`) instead of the
  hard-coded `4df7d539…65144`. Record the canonical build path in the docs.
  This removes the false failure without weakening the "reproducible from
  source at a stated path" claim.

- **If the deployed ID must remain fixed:** pin the build directory so every
  environment builds at the same absolute path (e.g. always extract/check out
  to `/home/runner/work/proclaim/proclaim`, including local verification),
  add `--locked` to the guest build, and re-derive/confirm that this canonical
  path yields `4df7d539…65144`. If it does not, update the expected ID in
  `REGISTRY_PROGRAM_ID`, `registry_core` constants, docs and cycle-harness
  assertions **only after** a green run at the canonical path — do not simply
  replace the asserted value with `5f80583a…d733` without that proof.

Do not change any repository until hypothesis 1 is confirmed by §4 step 1.

## 6. Honest limitations

- We could not read the failing step's log (GitHub job logs require
  authentication); the two IDs are taken from the problem statement and local
  staged docs.
- We could not build at `/home/runner/work/proclaim/proclaim` because the scope
  permits writes only under this investigation directory; therefore CI's exact
  `5f80583a…d733` was not reproduced here.
- No ID other than those explicitly observed above is claimed. The rebuild ID
  `daf7b2cd…35f2d` is a genuine observation from this investigation, not the CI
  result.
