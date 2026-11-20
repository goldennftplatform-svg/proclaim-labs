SPDX-License-Identifier: MIT AND Apache-2.0

# LP-0021 Phase 4 — Local Provider Protocol and Correlated Transport

A dependency-free, pure-Rust Cargo workspace that adds a precise, testable
**provider protocol** to the LP-0021 local wallet model: versioned envelopes,
opaque correlation ids, compact deterministic text serialisation, a fixed error
taxonomy, and an in-memory transport stack with explicit correlation checks,
bounded retries, cancellation, virtual-time timeouts and a narrow audit log.
There are no third-party crates, no path dependencies outside this workspace,
no git operations, no keys, tokens, credentials, deployment or network I/O.

## HONESTY / LIMITATIONS — read this first

- **In-memory transport is an explicit local model.** `InMemoryTransport`
  returns scripted strings from a local map. It is not a provider client, not a
  network stub and not a simulation of a real service.
- **No LEZ, chain, sequencer, testnet 0.4 or persistence.** Nothing is
  submitted, broadcast, confirmed or stored outside the process.
- **No keystore, no real approvals and no real signatures.** `approve` and
  `sign_message` produce local result strings only. The conformance line
  `sign=local-model-only` is deliberately not a signature.
- **No real users or accounts.** Payloads are opaque text; the protocol carries
  them verbatim but never interprets, authenticates or audits them.
- **No wall-clock time and no sleeping.** Backoff and timeout use a
  deterministic virtual clock (`RetryPolicy`), so results are reproducible.
- **Errors cannot leak handles.** `ProtocolError` is a fixed set of unit
  variants with no free-form fields. Audit entries contain exactly three
  fields: method, request id and outcome. Tests inject a handle-shaped string
  into payloads and assert it never appears in errors or audit output.
- **NOT criterion-complete for LP-0021.** This is a local protocol model only,
  not adoption evidence and not F1/F2/F3 satisfaction on a real chain.

## Workspace layout

| Path | Purpose |
| --- | --- |
| `Cargo.toml` | Workspace root; `resolver = "2"`, shared metadata, `MIT AND Apache-2.0`. |
| `Cargo.lock` | Generated lockfile; only the three local crates, zero external dependencies. |
| `crates/protocol-core/Cargo.toml` | Manifest for `protocol-core` (no dependencies). |
| `crates/protocol-core/src/lib.rs` | `RequestId`, `ProtocolVersion`, `Method`, `Payload`, `Request`, `Response`, `ProtocolError`, deterministic text codec; 18 tests. |
| `crates/provider-transport/Cargo.toml` | Manifest for `provider-transport`; local `protocol-core` path dependency only. |
| `crates/provider-transport/src/lib.rs` | `Transport`, `InMemoryTransport`, `OfflineTransport`, `RetryPolicy`, `ReliableTransport`, `AuditLog`; 20 tests. |
| `crates/conformance/Cargo.toml` | Manifest for the `conformance` binary; local path dependencies only. |
| `crates/conformance/src/main.rs` | Scripted four-method exchange, transient failure/retry, correlation probe and readable transcript; 7 tests. |
| `check_fmt.sh` | Helper for the exact fmt command. |
| `check_clippy.sh` | Helper for the exact clippy command. |
| `check_test.sh` | Helper for the exact test command. |
| `run_checks.sh` | Runs the three exact checks and prints exit codes. |
| `run_conformance.sh` | Prints the conformance transcript. |
| `count_tests.sh` | Counts `#[test]` functions in this workspace. |
| `clean.sh` | Removes only this workspace's `target/`. |
| `README.md` | This document. |
| `RESULT.md` | Exact verification results and transcript. |

Every authored `.rs`, `.toml` and `.md` file begins with
`SPDX-License-Identifier: MIT AND Apache-2.0` (comment-prefixed where the file
format requires it). Shell helpers carry it too.

## Modelled rules

### `protocol-core`

- `RequestId` is opaque (`r0000000000000001` text form). `RequestId::new()`
  allocates a fresh process-local id; `from_seed` exists for deterministic
  scripts and tests. Its numeric representation is private.
- `ProtocolVersion::V1` is the only supported version. `negotiate()` accepts
  `v1` and rejects every other offered number with `UnsupportedVersion`;
  malformed version tokens are `InvalidPayload`.
- `Method` has exactly five variants: `list_accounts`, `get_balance`,
  `approve`, `sign_message`, `transfer`. Unknown names yield `UnknownMethod`.
- Text forms are deterministic and unambiguous through escaping of `\`, `|`,
  newline and carriage return:
  - request: `REQ|r0000000000000001|v1|get_balance|account=local-1`
  - ok response: `RES|r0000000000000001|OK|balance=0`
  - error response: `RES|r0000000000000001|ERR|DENIED`
- `ProtocolError` is `UnsupportedVersion | UnknownMethod | Timeout | Cancelled
  | Denied | Transport | InvalidPayload`. Only `Transport` and `Timeout` are
  classified transient; `Cancelled` is an explicit decision, never a retry.
- A response is valid only when exactly one of result/error is present.

### `provider-transport`

- `InMemoryTransport::send` returns a correlated local response, or a scripted
  transient `Transport` failure. Failures can be global or targeted to one
  request id; response ids can be overridden to probe correlation.
- `OfflineTransport` explicitly fails every call with `Transport`; it never
  fabricates a success.
- `ReliableTransport` validates the version before sending, correlates every
  accepted response against its request id, rejects duplicate responses for
  completed ids, retries only transient errors, honours cancellation before or
  during a response, and enforces a virtual timeout budget.
- Backoff is deterministic and bounded by `RetryPolicy`: attempt 1 retry waits
  `base_backoff_ms`, later retries double up to `max_backoff_ms`. No sleeping
  occurs. `max_attempts` is clamped to at least 1.
- `AuditLog` entries are `(Method, RequestId, AuditOutcome)` only. Rendered
  lines look like `audit|approve|r0000000000000003|success`.

## Environment

Developed and verified in WSL Ubuntu with:

| Tool | Version |
| --- | --- |
| cargo | 1.100.0 (3acc79c0c 2026-11-05) |
| rustc | 1.100.0 (0ef1bc386 2026-11-09) |
| rustfmt | 1.11.0-stable (0ef1bc3860 2026-11-09) |
| clippy | 0.1.100 (0ef1bc3860 2026-11-09) |

Before running commands:

```sh
source ~/.cargo/env
export CARGO_BUILD_JOBS=2
```

`RUSTFLAGS` was intentionally left unset. No Docker was used.

## Exact verification commands and results

Run from this directory:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

| Command | Exit | Result |
| --- | --- | --- |
| `cargo fmt --all -- --check` | 0 | Clean; no formatting diff. |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | No warnings. |
| `cargo test --workspace` | 0 | 45 passed, 0 failed. |

`bash ./run_checks.sh` reported `FMT_EXIT=0`, `CLIPPY_EXIT=0`, `TEST_EXIT=0`.
Test distribution: 18 (`protocol-core`) + 20 (`provider-transport`) + 7
(`conformance`) + 0 doc-tests = **45 `#[test]` functions** (requirement: 25).

## Conformance transcript

Obtained with `cargo run -p conformance --quiet` (exit 0):

```text
LP-0021 provider-protocol conformance (in-memory model only)
no LEZ / chain / keystore / real signature / real provider
1. list_accounts req=r0000000000000001 -> OK RES|r0000000000000001|OK|accounts=0 attempts=1 verified_id=true
   transient failure -> retry #1 (backoff 10ms virtual, no sleep)
2. get_balance req=r0000000000000002 -> OK RES|r0000000000000002|OK|balance=0 attempts=2 verified_id=true
3. approve req=r0000000000000003 -> OK RES|r0000000000000003|OK|approval=local-record attempts=1 verified_id=true
4. sign_message req=r0000000000000004 -> OK RES|r0000000000000004|OK|sign=local-model-only attempts=1 verified_id=true
Ordering: approve before sign_message = YES
5. correlation probe req=r0000000000000009 with response id=r0000000000000001 -> REJECTED (no response accepted without a matching request id)
Audit (method + request id + outcome only):
  audit|list_accounts|r0000000000000001|sent
  audit|list_accounts|r0000000000000001|success
  audit|get_balance|r0000000000000002|sent
  audit|get_balance|r0000000000000002|transient_failure
  audit|get_balance|r0000000000000002|retry_scheduled
  audit|get_balance|r0000000000000002|sent
  audit|get_balance|r0000000000000002|success
  audit|approve|r0000000000000003|sent
  audit|approve|r0000000000000003|success
  audit|sign_message|r0000000000000004|sent
  audit|sign_message|r0000000000000004|success
  audit|transfer|r0000000000000009|sent
  audit|transfer|r0000000000000009|correlation_mismatch
Checks: four_successes=YES retry=YES ordering=YES correlation=YES audit=YES
CONFORMANCE PASSED
```

The one transient failure is injected only into request id 2; the retry is
visible as `attempts=2` plus the `transient_failure`/`retry_scheduled` audit
lines. The approval at step 3 precedes signing at step 4. Step 5 receives a
response bearing a previously used id and is rejected, so no response is
accepted without a matching request id.

## Not implemented

No real provider, network transport, LEZ zone, chain, sequencer, testnet,
keystore, real approvals, real signatures, real accounts, real users or
persistence. No wall-clock timeouts or real sleeping. No CLI beyond the
conformance printer, no cross-workspace integration, no adoption evidence and
no LP-0021 criterion-completeness claim.

## Cleanup

`bash ./clean.sh` removes only this workspace's `target/` directory. Build
artifacts are not part of the staged deliverable.
