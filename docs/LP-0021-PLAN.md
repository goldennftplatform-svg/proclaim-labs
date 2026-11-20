<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# LP-0021 — Design & Implementation Plan (design task only)

**Date:** 2026-11-19
**Status:** Design document. No repository edits, commits, pushes, tokens, or PRs were made. The only file produced by this task is this document.
**Prize:** LP-0021: LEZ Wallet and Provider SDK `[OPEN]`, **Total Prize: $20,000**, **Effort: X-Large**.
**Baseline codebase:** `proclaim` at `/home/xps/logos-work/registry` (MIT AND Apache-2.0).

---

## 0. Source provenance and verification status

| Source | How obtained | Verified? |
|---|---|---|
| `prizes/LP-0021.md` (full, verbatim) | `raw.githubusercontent.com/logos-co/lambda-prize/master/prizes/LP-0021.md` | Yes |
| `README.md` (full rules) | raw `master/README.md` | Yes |
| `prizes/LP-0022.md`, `prizes/LP-0001.md` (skimmed in full) | raw `master` | Yes |
| `prizes/LP-0023.md` (full dependency spec) | raw `master` | Yes |
| Open PR list (15 open) | HTML `github.com/logos-co/lambda-prize/pulls?q=is%3Apr+is%3Aopen` | Yes, HTML list as of 2026-11-19 |
| PR diffs #181, #188, #189, #137 | `github.com/.../pull/<n>.diff` | Yes |
| GitHub REST API `/pulls?state=open` | attempted | **No — HTTP 403 (rate limited)**. All PR claims below rest on the HTML list + `.diff` endpoints, not the API. PR mergeability, CI state, review state and exact file lists beyond the returned diffs are **unverified**. |

### 0.1 Open PRs that can change LP-0021 criteria (all OPEN, none merged)

| PR | Opened | Effect on this plan |
|---|---|---|
| **#181** `LP-0021/LP-0022: replace faucet mini-app with open funding path and EmPoWer` | 2026-10-12 | **Directly amends LP-0021.** Deletes the **Faucet mini-app** success criterion, deletes the faucet worked-example clause from SDK docs, removes the faucet from Scope and Submission Requirements, and adds a note: *"How they obtain funds is not a criterion of this prize: it is left to the submission to choose whatever gives the best UX and adoption."* Also permits EmPoWer / atomic-swap / bridge liquidity. |
| **#189** `Release integrity and security requirements for submissions` | 2026-11-13 | Adds programme-wide **Release integrity and security** rules and a `Security Tier` line. A wallet is Tier 2 ("keys or funds"): threat model, approval enforced in the **core module**, no plaintext keys, spend caps outside the agent, **independent review / bug-bounty window before adoption counts**, signed `.lgx` from a declared signer DID, immutable GitHub releases, release-integrity manifest with `rootHash`, and *"Adoption counts only from signed releases in the declared catalog, on testnet."* The current master LP-0021 has **no** Security Tier line and **no** release-integrity requirement; treat this as a high-probability pending obligation. |
| **#188** `Update for lez 0.4, and updated idl` | 2026-11-02 | Amends **LP-0023**, not LP-0021: replaces SPEL with *"a LEZ actor program (an `actor_entrypoint!` handler taking Borsh-encoded messages)"*, adds the `lez-idl` format and requires tooling to generate/encode/decode IDLs, **"plan and simulate a call against current chain state before it is sent: which actors it must declare, which accounts must sign, whether it applies or reverts, and its cycle count and fee"**, and compute a program's `image_id` from a local build and compare it with the deployed program header. This is the intended substrate for LP-0021's human-readable approval summary and source-verification badge. |
| **#137** `Define Adoption criteria` | 2026-09-02 | Adds `ADOPTION.md` (guidance only in the returned diff; it does not edit LP-0021). Signals the programme's adoption philosophy: third-party-only, A4 downstream modules, B2 distinct-account on-chain activity, B3 sustained-window modifier, D3 seeded demand pool, F1 "Not counted" (stars, downloads, likes, etc.). |

Other 11 open PRs (solutions for LP-0018/0026, new prizes LP-0027/0028/0029, theming, adoption-adjacent drafts) were inspected by title only and do not visibly amend LP-0021.

**Design consequence:** the plan below implements the **current master spec** (faucet included) but isolates funding behind a `FundingProvider` trait so that PR #181 can be absorbed by swapping one adapter. It treats the #189 Tier 2 obligations and #188's `lez-idl` shape as the likely target, without assuming they have merged.

---

## 1. Verbatim anchors from LP-0021 (quote where it matters)

- *"the wallet operates end-to-end against the canonical Logos LEZ zone on Logos testnet 0.4."*
- *"the user is shown a selection prompt (which account to use) and an approval prompt (what is being signed/sent) before anything is signed or submitted."*
- *"For smart-contract interactions (including token approvals), the prompt must show a human-readable summary of the call's effects: assets/approvals granted or moved and to which program/account, not just raw calldata, so the user can catch unexpected outflows or unlimited approvals before signing."*
- *"The approval prompt must show the user the program's source-verification status for the program being called. A program registry that meets or is similar to the specifications of LP-0023 will be suitable."*
- *"This means **a standard LEZ program that stores each testimonial on-chain**."* — the only on-chain program the spec actually requires.
- *"Non-fungible (NFT) assets are out of scope — see Out of Scope."* and *"Designing or implementing the token program itself — the wallet integrates with the existing/reference implementation."*
- *"Beyond the user's configured sequencer and Logos LEZ node, the wallet must not depend on any third-party web service to function."*
- *"All of the following are required in addition to the FURPS criteria above."* and *"hitting a raw number is not by itself sufficient"*.
- *"A single burst of activity does not qualify, however large."*
- *"Builders are encouraged to release early and start accruing adoption while continuing to develop, rather than treating the build and the adoption push as sequential."*
- Evaluation: *"first-come-first-served"*; *"Evaluators will independently clone the repository and run the demo script from a clean environment; the script must succeed without modification."*; max 3 submissions, at most one submission/review per week.

**Master spec has no `Security Tier` line** (that is PR #189 territory). Do not invent one into the criteria count.

---

## 2. Requirement matrix — 32 rows (26 Success Criteria + 6 Adoption)

Legend: **Reuse** = exists in `proclaim` today (possibly with adaptation); **New** = must be built; **Blocked** = cannot be completed now (testnet 0.4, external users, or the 2-month clock). A row can have all three.

### 2.1 Functionality (8)

| # | Criterion (short verbatim anchor) | Reuse from proclaim | New | Blocked |
|---|---|---|---|---|
| F1 | **LEZ assets** — *"own, send, and receive native and token-program assets on a LEZ zone, for both private and public accounts"* | Borsh types + PDA helpers from `registry_core`; read-only chain-query/FFI patterns from `registry_ffi`, `clients/js`, `registry-api`. | Wallet core: key management, `AccountKind::{Public,Private}`, balance aggregation, tx build/sign/submit, native transfer. | Exact testnet-0.4 shielded-account API and native-asset layout unverified; end-to-end only on 0.4. Local-sequencer slice possible now. |
| F2 | **Token program** — fungibles only, *"from both public and private accounts"* | LEZ program integration pattern (Borsh msgs, PDA/IDL) from the 6-instruction registry; `verify-source.sh` pattern for byte-match confidence. | `wallet-token` adapter over the reference token program IDL; balance/mint/transfer/allowance; **no token-program design** (explicitly out of scope). | Token program IDL/ABI for testnet 0.4 not in hand; NFT paths deliberately skipped (LP-0001). |
| F3 | **Multiple accounts (LEZ)** — create/manage/switch public + private | Qt settings/config + FFI dispatch patterns in `ui/registry/`; Borsh/PDA helper idioms. | `AccountStore`, account creation/import, switching, per-account labels, encrypted persistence. | Nothing hard-blocked; shielded key generation needs the 0.4 crypto stack for final validation. |
| F4 | **Wallet Provider SDK** — *"request account access, read balances/state, and propose transactions"* | `clients/js` dependency-free JS/TS client pattern; `registry_core` query/search types; `registry_ffi` C-ABI + generated header pattern. | JSON-RPC-style provider protocol, typed request/response schema, capability/permission store, session model, docs. | Nothing hard-blocked; can be fully specified and mock-tested today. |
| F5 | **Wallet selection & approval flow** — selection + approval *"before anything is signed or submitted"* | QML `Main.qml` + C++ FFI dispatch from `ui/registry/`. | `ApprovalCoordinator` state machine, dApp identity display, per-account/per-capability grants, audit log; enforcement must live in the core module (PR #189 Tier 2). | Nothing hard-blocked; test with a mock dApp now. |
| F6 | **Testimonial mini-app** — *"a standard LEZ program that stores each testimonial on-chain"*; custom text, optional username, unique submission identifier | On-chain-program scaffolding pattern from the registry (SPEL today; see #188 actor/`actor_entrypoint!` risk); `clients/js`; `.lgx` build from `docs/LGX-BUILD.md`. | New on-chain `testimonial_store` program + IDL; mini-app; **fixed submission ID** embedded in every record; worked example. | On-chain deployment and 150-account adoption need 0.4 + users. Program can be written and tested on a local sequencer first. |
| F7 | **Faucet mini-app** — *"requests testnet funds from the LEZ faucet, using the SDK to let the user select which account receives them"*; success/failure + rate limiting | SDK selection flow from F4/F5; `clients/js`. | `FundingProvider` abstraction + HTTP faucet client, rate-limit/backoff, receipt reporting. | Faucet endpoint/rate limits on 0.4 unknown; **PR #181 would delete this criterion entirely** — build as a swappable adapter only. |
| F8 | **Testnet 0.4** — *"operates end-to-end against the canonical Logos LEZ zone on Logos testnet 0.4"* | Local sequencer harness pattern is unknown; toolchain (Rust risc0, Qt6, CMake, libsodium) present. | Deployment, endpoint config, smoke scripts. | **HARD BLOCKED** — no testnet 0.4 access assumed today. |

### 2.2 Usability (7)

| # | Criterion | Reuse | New | Blocked |
|---|---|---|---|---|
| U1 | Module/SDK reusable by other Logos modules and third-party dApps for balances, access, transfers, contract interactions | `registry_core` + `registry_ffi` + `clients/js` + `ui/registry/` module shape. | `wallet-provider` API, permissions, worked examples. | None today. |
| U2 | Basecamp app GUI with local build instructions and loadable assets | **Strong**: `ui/registry/` (`Main.qml`, C++ FFI backend), `docs/LGX-BUILD.md`, signed `.lgx` via local `lgx` CLI. | Wallet-specific QML views, approval dialogs, loadable asset bundle. | None locally (Qt6 present); macOS/Linux Windows/Android matrix not all testable in WSL. |
| U3 | Module catalog fork of `logos-modules-release-base` + `logos-modules-release-action`; publish `logos-repo.json` URL | Local `.lgx` signing already exists; reuse artifact pipeline. | Catalog fork on a **GitHub organisation account**, release workflow, signer DID in `trustedSigners`, immutable releases. | External infra + signer key + org account; PR #189 makes this mandatory-grade. Start setup early but cannot finish offline. |
| U4 | Prompts identify dApp, account, asset/amount/destination; for contracts *"a human-readable summary of the call's effects ... not just raw calldata"* | Registry metadata types + `search` in `registry_core`; QML prompt patterns. | IDL-driven call decoder + effects summariser; explicit unlimited-approval warning. | Depends on `lez-idl` decode/simulate tooling (LP-0023 + PR #188). Can build against a mock IDL now. |
| U5 | **Program source verification** — approval prompt shows verification status; LP-0023 registry suitable | **Strong**: `proclaim` *is* an LEZ program registry; `scripts/verify-source.sh` clean-room verifier; PDA/search types. | `wallet-registry` resolver, `VerificationStatus` model, local rebuild+compare, status badge in QML, graceful degradation. | Needs a deployed LP-0023 registry on 0.4 for live data; build adapter + mock now. PR #188 changes verification to `image_id` vs program header — verify before coding. |
| U6 | **UX quality** — *"coherent navigation across accounts, comprehensible transaction previews, and clear error/pending states"* | `ui/registry/` QML idioms and theme. | Wallet UX, approved/pending/error states. | Final judgement needs real users + 0.4. |
| U7 | Display estimated gas usage and gas used *"if available on testnet v0.4 (as currently planned)"* | PR #188's planned simulate/cycle-count tooling (unmerged). | Gas estimate + used fields in prompt and history. | **Conditional/blocked** — literally gated on 0.4 availability. |

### 2.3 Reliability (4)

| # | Criterion | Reuse | New | Blocked |
|---|---|---|---|---|
| R1 | State survives restarts/network drops without corruption | Qt config persistence patterns; Borsh serde discipline. | Versioned encrypted state file, atomic writes, migration, reconnect logic. | None; fully testable locally. |
| R2 | No sign/submit or private-balance read without *"explicit user approval for that account"* | FFI dispatch boundary from `ui/registry/`. | Permission engine + enforcement in **core module**, not just QML; negative tests. | None today; PR #189 strengthens to Tier 2 core-module enforcement. |
| R3 | **No mandatory external services**; disclosure + disable + opt-in analytics; no transmit of addresses/balances/tx contents without opt-in | `registry-api` is a *read-only optional* surface — good precedent; keep it non-mandatory. | Local-first design, network capability flags, zero telemetry by default. | None; must be a design invariant from day 1. |
| R4 | **Remote storage uses Logos Storage**, client-side encryption if remote persistence is needed | libsodium in toolchain; encryption design from R1. | Logos Storage adapter (or deliberately no remote persistence). | Logos Storage API/availability unverified locally; stub behind trait now. |

### 2.4 Performance (1)

| # | Criterion | Reuse | New | Blocked |
|---|---|---|---|---|
| P1 | *"adequate UX"*, judged subjectively via adoption | `ui/registry/` responsiveness baseline. | Local perf smoke tests (account switch, list render, prompt latency). | Final judgement blocked on 0.4 + real users. |

### 2.5 Supportability (6)

| # | Criterion | Reuse | New | Blocked |
|---|---|---|---|---|
| S1 | Deployed and tested against testnet 0.4 (the Logos LEZ zone) | — | Deployment scripts, configured endpoint. | **HARD BLOCKED**. |
| S2 | Modules built with `logos-module-builder` (`mkLogosModule`) | `.lgx` build pipeline in `docs/LGX-BUILD.md`. | Wire `mkLogosModule` into both core and ui module builds. | None locally. |
| S3 | Platform support *"recommended, not a separate build gate"*: Linux/macOS; Windows+Android from 0.4 | Existing Linux/`.lgx` build. | macOS CI runner, later Windows/Android variants. | No macOS host in WSL; macOS proven through **A6** adoption instead. |
| S4 | E2E tests against a real sequencer (standalone mode) in CI; green default branch | CI scaffold (fmt/clippy, 7 Rust + 8 JS tests). | Standalone-sequencer harness, e2e specs, CI job. | Possible locally **only if** a standalone sequencer binary is available without Docker — **unverified**; testnet leg blocked. |
| S5 | README: setup, account management, CLI + Basecamp step-by-step | `proclaim` README conventions. | New wallet README. | None. |
| S6 | SDK docs cover testimonial **and faucet** as worked examples of connect → selection → approval → tx | `clients/js` docs pattern. | Two worked-example chapters. | Faucet chapter at risk from PR #181; keep generic *funding* chapter. |

### 2.6 Adoption (6) — *"All of the following are required"*

| # | Criterion | Reuse | New | Blocked |
|---|---|---|---|---|
| A1 | **10 third-party developers** each shipping a functional **ui-type** Basecamp app using the SDK; genuine commit history; independent of each other and of submitter | Catalog/module pattern to hand to adopters; starter template from `ui/registry/`. | Outreach, starter repo, integration support, evidence links. | **External users** — cannot start count now; need SDK stable + signed catalog (PR #189). |
| A2 | **150 on-chain testimonials** via the mini-app on the official zone, *"I use wallet XYZ"* or clear custom text, ≥150 distinct accounts, prior testnet activity weighted | `testimonial_store` program + mini-app from F6. | Submission ID, attribution evidence, per-account tally. | **HARD BLOCKED** on testnet 0.4 + real users; no meaningful progress possible offline. |
| A3 | **Sustained activity** — 150 over *"at least 2 months, with at least 30 new testimonials in each of those months"*; timestamped on-chain | On-chain timestamps from F6. | Monthly evidence report. | **HARD BLOCKED — the 2-month clock cannot start until the first on-chain testimonial exists.** PR #189 would additionally delay the clock until an independent review/bug-bounty window completes. |
| A4 | **30 Discord testimonials** vouching, describing actual use; account history checked | — | Community presence in #builder-hub, evidence links. | **External users.** |
| A5 | **30 Twitter/X testimonials**, same standard; screenshots encouraged | — | Same. | **External users.** |
| A6 | **macOS coverage** — evidence must include macOS users/installs as well as Linux | Linux build + `.lgx` pipeline. | macOS-compatible signed release. | **External users + no macOS host**; needs macOS CI or adopters. |

**Counts:** Functionality 8 + Usability 7 + Reliability 4 + Performance 1 + Supportability 6 = **26 success criteria**; **6 adoption items**; **32 total**. Additional non-counted obligations: public dual-licensed repo (MIT **and** Apache-2.0), narrated video walkthrough, per-adoption evidence, FURPS self-assessment, solution PR in `lambda-prize`, clean-environment demo script.

---

## 3. Architecture

### 3.1 Guiding decisions

1. **No on-chain wallet program is required.** The spec only mandates a *standard LEZ program that stores testimonials*. Keys stay client-side in an encrypted core module. A smart-contract wallet would add attack surface and is not a criterion — out of scope.
2. **Local-first.** Sequencer + LEZ node are the only mandatory network dependencies (R3). The registry, IDL store, faucet and any analytics are optional adapters with explicit user disclosure/disable.
3. **Approval is a protocol-level invariant**, enforced in the core module before any sign/read of a private account — not a QML-only guard (R2, PR #189 Tier 2).
4. **Adapter boundaries absorb pending spec churn**: `FundingProvider` (PR #181), `ProgramRegistry` + `IdlProvider` (PR #188), `RemoteStore` (Logos Storage), `AccountBackend::{Public,Shielded}`.
5. **One fixed submission ID** is generated at project start and compiled into the testimonial program and docs; it cannot change later (F6, A2, A3).

### 3.2 Crate / module layout

```
lez-wallet/                         # new workspace; dual-licensed MIT + Apache-2.0
├─ Cargo.toml                       # workspace, pinned Cargo.lock (PR #189 baseline)
├─ programs/
│  ├─ testimonial_store/            # F6: the only required on-chain program
│  │  ├─ src/lib.rs                 # actor_entrypoint!/Borsh msgs if PR #188 model wins;
│  │  │                            #   SPEL v0.7.0 handler if current LEZ 0.2.4 model holds
│  │  ├─ idl/                       # generated lez-idl, checked in + reproducible
│  │  └─ tests/                     # local-sequencer integration tests
│  └─ (no wallet program — deliberate)
├─ crates/
│  ├─ wallet-types/                 # AccountId, AccountKind, Asset, TxRequest, Capability,
│  │                                #   VerificationStatus, Permission, ApprovalRequest
│  ├─ wallet-keystore/              # libsodium encrypted-at-rest; OS keystore hook; unlock
│  ├─ wallet-core/                  # AccountStore, policy engine, ApprovalCoordinator,
│  │                                #   versioned atomic state, audit log
│  ├─ wallet-chain/                 # sequencer RPC, LEZ node client, submit, gas adapter
│  ├─ wallet-token/                 # fungible token program adapter (Public + Shielded)
│  ├─ wallet-registry/              # LP-0023 consumer: resolve program ID -> metadata +
│  │                                #   source-verification status; reuse registry_core
│  ├─ wallet-idl/                   # thin adapter over lez-idl decode/encode/simulate/image_id
│  ├─ wallet-provider/              # SDK protocol, session/capability store, request router
│  ├─ wallet-ffi/                   # C ABI + generated header (mirrors registry_ffi)
│  ├─ wallet-cli/                   # `lgs wallet ...`
│  └─ wallet-e2e/                   # standalone-sequencer harness + fixtures
├─ clients/js/                      # dependency-free TS client (mirrors proclaim clients/js)
├─ apps/
│  ├─ testimonial/                  # F6 mini-app (ui-type module + thin web/shell front-end)
│  ├─ funding/                      # F7 adapter; faucet now, EmPoWer/bridge if #181 lands
│  └─ basecamp/                     # U2/S2: Qt/QML core + ui modules
│     ├─ core/                      # holds keys; enforces approval (Tier 2)
│     ├─ ui/
│     │  ├─ src/backend/            # C++ WalletBackend -> wallet_ffi dispatch (ui/registry pattern)
│     │  ├─ qml/Main.qml            # adapted from ui/registry/Main.qml
│     │  └─ qml/components/         # AccountSwitcher, ApprovalDialog, VerificationBadge, TxPreview
│     └─ docs/LGX-BUILD.md          # copied/adapted from proclaim
├─ scripts/verify-source.sh         # fork/adapt clean-room verifier for wallet-registry
├─ docs/                            # SDK.md, THREAT-MODEL.md (Tier 2), RELEASE-INTEGRITY.md,
│                                   # SPEC-DELTA.md (tracks #181/#188/#189/#137)
└─ .github/workflows/               # CI: fmt/clippy, Rust+JS tests, standalone e2e, signed .lgx
```

### 3.3 Key APIs (Rust, mirrored in C FFI and TS)

```rust
// wallet-types
pub enum AccountKind { Public, Shielded }
pub enum Capability { AccountAccess, ReadBalances, ProposeTransaction, GetVerificationStatus }
pub struct Account { id: AccountId, kind: AccountKind, label: String, zone: ZoneId }
pub struct Asset { program_id: ProgramId, mint: Mint, amount: u128, decimals: u8 }
pub struct TxRequest { dapp_id: DappId, session: SessionId, program_id: ProgramId,
                       message: Vec<u8>, declared_actors: Vec<ActorId>,
                       fee_payer: AccountId, value: Option<Asset> }
pub enum VerificationStatus {
    Unregistered, RegisteredUnverified,
    LocallyVerified { source_commit: String, image_id: String },
    DeployerClaimUnverified, Mismatch,
}

// wallet-provider (protocol; JSON over in-process channel / local socket)
wallet_connect        { dapp_id, capabilities[] }            -> session_id
wallet_getAccounts    { session }                             -> Vec<AccountRef>   // no addresses unless granted
wallet_getBalances    { session, account, assets[] }          -> Vec<Asset>        // approval for Shielded
wallet_getVerificationStatus { program_id }                   -> VerificationStatus
wallet_proposeTransaction    { TxRequest }                    -> request_id        // -> approval prompt
wallet_approve / wallet_reject { request_id, account }        -> result
wallet_getEffectsPreview { request_id }                       -> EffectsSummary    // human-readable
wallet_revoke         { session, capability }                 -> ()

// wallet-core (enforced before sign / private read)
fn authorize(session: SessionId, cap: Capability, account: &AccountId,
             effects: &EffectsSummary) -> Result<ApprovalToken, Denied>;

// wallet-registry (LP-0023 consumer)
trait ProgramRegistry {
    fn resolve(&self, program_id: ProgramId) -> Result<ProgramMeta, RegistryError>;
    fn verification_status(&self, program_id: ProgramId) -> VerificationStatus;
}
```

`EffectsSummary` fields: requesting dApp, target account (+ zone), program name/version/author, asset, amount, destination, **approvals granted/moved and to which program/account**, explicit *unlimited approval* flag, gas estimate/used (conditional U7), verification badge, raw-calldata fallback.

### 3.4 Consuming LP-0023 source verification in the approval flow

1. `wallet-proposeTransaction` receives a `program_id`.
2. `wallet-registry::resolve` reads on-chain metadata through PDAs derived with the same helper style as `registry_core` (name, version, author, description, IDL pointer, code pointer, registration timestamp, deployer-signed flag).
3. `wallet-idl` decodes the Borsh message using the registered IDL (or, if PR #188 lands, the `lez-idl` tooling that can *"plan and simulate a call against current chain state before it is sent"* — declare actors, signers, apply/revert, cycle count and fee). The simulated diff becomes `EffectsSummary` — this is what satisfies the *"not just raw calldata"* clause (U4).
4. `verification_status`:
   - no on-chain entry → `Unregistered`;
   - entry present but no source claim → `RegisteredUnverified`;
   - source commit + build config present → run adapted `verify-source.sh`: fetch source via pointer, rebuild with recorded config, compute `image_id` from the local build and compare to the deployed program header (the #188 mechanism); on match, cache `(program_id, image_id, source_commit)` locally and let the user mark it `LocallyVerified` **for themselves only**;
   - mismatch → `Mismatch` with a hard warning in the prompt.
5. QML `VerificationBadge` shows status + source commit + deployer/third-party distinction. Cache is keyed by program `image_id` so a redeploy invalidates the badge.
6. **Graceful degradation** (mirrors LP-0023 reliability criterion): unresolvable IDL → still show on-chain metadata, badge and raw calldata with an explicit *"cannot render effects — IDL unavailable"* warning; never silently fail the whole view.
7. Nothing here is mandatory for the wallet to function: if the registry is unreachable, the user can still approve with a clear "unverified / unknown program" state (R3).

### 3.5 Reusing the `ui/registry/` Qt/QML pattern

- Copy the module skeleton: `src/backend/` C++ class with `Q_INVOKABLE` methods that marshal to the generated `wallet_ffi` header — identical dispatch shape to `RegistryBackend`.
- Reuse `Main.qml` navigation, theme, list/detail components; replace registry entries with accounts/assets and add `ApprovalDialog.qml` (the F5 gate), `AccountSwitcher.qml` (F3), `TxPreview.qml` and `VerificationBadge.qml` (U4/U5).
- Reuse `docs/LGX-BUILD.md` and the local `lgx` CLI signing step verbatim; add `mkLogosModule` (S2).
- Bind QML models to FFI signals for `approvalRequested`, `txPending`, `txFinalized`, `networkLost`; all state transitions are recorded in the core-module audit log.
- Keep the UI disposable: approval decisions are enforced in `apps/basecamp/core` + `wallet-core`, so a compromised or buggy QML layer cannot sign (R2, PR #189 Tier 2).

---

## 4. Phased, prioritized plan

### Phase 0 — TODAY, no testnet 0.4 (pure spec / Rust / local)

**The first 3 concrete actions:**

1. **Spec-delta baseline + freeze the submission ID.** Write `docs/SPEC-DELTA.md` recording PR #181/#188/#189/#137 with exact quoted changes, decide the generic `FundingProvider` abstraction so dropping the faucet (F7) is a one-adapter change, and fix the unique LP submission identifier used in the testimonial program, all docs and all evidence. Nothing external required.
2. **Stand up `wallet-types`, `wallet-keystore`, `wallet-core` in pure Rust** with libsodium-backed encrypted persistence, versioned atomic state, `AccountStore`, and the `ApprovalCoordinator` state machine plus unit/property tests (R1, R2, F3). No network, no testnet.
3. **Write the Wallet Provider SDK protocol spec + JSON schema and a dependency-free TS client with a mock provider** (mirroring `clients/js`), including capability grants, per-account approval, and negative tests proving a dApp cannot sign or read a shielded balance without approval (F4, F5, R2).

Also in Phase 0 (parallel, still no testnet): feasibility spike — can a standalone LEZ sequencer run in WSL without Docker (S4)? Fork/adapt `scripts/verify-source.sh` into `wallet-registry` with a mocked registry entry (U5). Draft `docs/THREAT-MODEL.md` skeleton for Tier 2 (PR #189).

### Phase 1 — Local vertical slice (public account, native asset)

- `wallet-chain` against a local/standalone sequencer; native send/receive; `wallet-cli`; gas adapter stub (F1, F8-local, S4, S5).
- Extend persistence across restarts/network drops; reconnect tests (R1).
- CI: extend existing fmt/clippy + 7 Rust + 8 JS tests with the new crates and local e2e.

### Phase 2 — Token, private accounts, first on-chain program

- `wallet-token` fungible adapter for public + shielded accounts (F2); approval-gated shielded balance reads (R2).
- Implement and locally deploy `testimonial_store` with the fixed submission ID; build the testimonial mini-app on the SDK (F6): full connect → selection → approval → tx.
- `FundingProvider` + mock faucet client with rate-limit handling (F7, behind the #181 swappable boundary).

### Phase 3 — Basecamp GUI, registry, UX

- Port `ui/registry/` pattern: C++ FFI backend + `Main.qml` + approval/selection/verification components (U2, U4, U5, U6).
- `wallet-registry` + `wallet-idl` integration; source-verification badge; graceful IDL degradation; gas display if the local sequencer exposes it (U7).
- `mkLogosModule` wiring, signed `.lgx` via `lgx`, local install smoke test (S2, U3-partial).

### Phase 4 — Release, security and docs hardening (overlaps Phase 3)

- Module catalog fork on a GitHub org, signer DID, immutable releases, release-integrity manifest with `rootHash`/SHA-256/variants, canonical install instructions (U3, PR #189).
- Tier 2 evidence: threat model, core-module approval enforcement, no plaintext keys, spend caps, independent-review / bug-bounty window scheduling (PR #189).
- README + SDK docs with testimonial and *funding* worked examples (S5, S6), FURPS self-assessment, narrated demo script, solution-PR evidence pack.

### Phase 5 — Testnet 0.4 + adoption (HARD-BLOCKED until 0.4 and external users exist)

- Deploy programs, switch endpoints, run e2e on the canonical zone `https://testnet.lez.logos.co` (F8, S1).
- **Start the 2-month clock the day the first on-chain testimonial lands** — release early, then keep shipping (A2, A3).
- Recruit 10 independent third-party ui-module developers (A1); drive 150 distinct-account testimonials at ≥30/month (A2/A3); 30 Discord + 30 X testimonials (A4/A5); ensure macOS evidence (A6).
- Only then open the solution PR with per-criterion evidence; respect 3-submission / 1-per-week limits.

### Start-now vs hard-blocked

| Can start now | Hard-blocked |
|---|---|
| Spec delta, submission ID, SDK protocol, pure-Rust core/keystore, approval state machine + negative tests, QML skeleton from `ui/registry/`, `verify-source.sh` adapter with mocks, README/SDK docs, CI scaffold, module-catalog setup groundwork, threat model | Testnet 0.4 end-to-end (F8, S1), 150 on-chain testimonials (A2), the 2-month clock (A3), all social/third-party adoption (A1, A4, A5, A6), faucet endpoint behaviour (F7), Logos Storage live integration (R4), gas figures (U7), macOS real-use evidence |

---

## 5. Effort estimate (honest, assumption-bound)

Estimates are engineering judgement, **not** facts, statuses or commitments. Assumes 2 Rust engineers + 1 Qt/QML engineer (part-time through Phase 3), no Docker, `CARGO_BUILD_JOBS=2` and the given `RUSTFLAGS` (slow RISC0 builds), and that testnet 0.4 becomes available during Phase 4.

| Phase | Engineer-weeks (range) | Notes |
|---|---|---|
| 0 | 1–2 | Spec + core types + keystore + TS mock client |
| 1 | 4–6 | Local sequencer integration, CLI, persistence, CI e2e |
| 2 | 6–10 | Token + shielded accounts, testimonial program + mini-app, funding adapter |
| 3 | 6–10 | Qt/QML module, IDL effects summary, verification badge, UX polish |
| 4 | 3–5 + external lead time | Catalog/signing/immutability/review window; not purely coding |
| **Build subtotal** | **~20–33 engineer-weeks (~5–8 engineer-months)** | Calendar ~4–6 months with 3 people in parallel |
| 5 (adoption) | ≥2 elapsed months + outreach lead time | Cannot be bought down; realistically 3–5 additional calendar months |

The $20,000 prize and `X-Large` effort label are quoted from the spec; no other amounts or timelines are asserted.

---

## 6. Top risks / unknowns

1. **PR #181 (faucet removal).** Building a dedicated faucet mini-app could be wasted. Mitigation: `FundingProvider` adapter + generic "funding" docs chapter.
2. **PR #189 (Tier 2 release integrity).** Signed `.lgx`, immutable releases, org-hosted catalog, `rootHash` manifest, **independent review before adoption counts**, and approval enforced in the core module are significant process and design obligations not yet in master LP-0021. Start the catalog/signing work in Phase 0–4, not Phase 5.
3. **PR #188 (LP-0023 platform shift).** SPEL → `actor_entrypoint!` + Borsh + `lez-idl` + `image_id` verification conflicts with `proclaim`'s SPEL v0.7.0 / LEZ v0.2.4 assumptions. Verify the actual testnet-0.4 program model before writing `testimonial_store`; keep a program-interface abstraction so the swap is local.
4. **Testnet 0.4 availability.** Hard blocker for F8, S1, A2, A3 and conditional U7. No amounts, dates or endpoint behaviour are assumed beyond the spec's `https://testnet.lez.logos.co`.
5. **Shielded account + token program APIs on 0.4 unverified.** Need actual IDLs; design behind `AccountBackend`/`wallet-token` traits. NFTs explicitly out of scope (LP-0001).
6. **LP-0023 deployment status.** Verification badge may have no live registry data on 0.4; graceful degradation is mandatory, and the badge's evidentiary value depends on another team's prize succeeding.
7. **Standalone sequencer in WSL without Docker (S4)** is unverified; if unavailable, the local e2e CI criterion cannot be proven before 0.4.
8. **Adoption is the real gate.** 10 independent module developers with genuine histories, 150 distinct accounts with prior activity, ≥30/month for 2 months, 30+30 vouching posts, macOS evidence — plus anti-Sybil inspection and PR #189's signed-release-only rule. *"hitting a raw number is not by itself sufficient."*
9. **Clock starts late if bug-bounty/review window applies.** PR #189 would delay when adoption starts counting for the evaluated build; schedule that review in Phase 4.
10. **macOS** cannot be validated from WSL; needs macOS CI or real adopters.
11. **Logos Storage** API/size limits are unverified; keep remote persistence optional and stub the adapter (R4).
12. **Licensing / supply chain:** all crates and apps must be MIT **and** Apache-2.0; pinned `Cargo.lock`; no plaintext keys in repo, logs, screenshots or demo video (PR #189 baseline).
13. **First-come-first-served + one review per week** rewards early, correct submission; do not burn a submission slot on an incomplete evidence pack.

---

## 7. Deliverable checklist for this design task

- [x] Full LP-0021 spec read verbatim; README rules read; LP-0022, LP-0001 skimmed; LP-0023 read as dependency.
- [x] Open PR list checked via HTML (API 403); relevant diffs read; unverified items flagged.
- [x] Requirement matrix: 26 success criteria + 6 adoption items = 32, each with reuse/new/blocked.
- [x] Concrete architecture: crates/modules, key APIs, LP-0023 verification in approval flow, `ui/registry/` QML reuse.
- [x] Phased plan 0–5 with the first 3 today-actions and a start-now / hard-blocked split.
- [x] Honest effort estimate and top risks; no invented amounts, statuses or criteria.
- [x] Exactly one output file written; no repo edits, commits, pushes, tokens or PRs.
