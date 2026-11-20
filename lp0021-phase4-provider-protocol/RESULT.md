SPDX-License-Identifier: MIT AND Apache-2.0

# RESULT — LP-0021 Phase 4 Local Provider Protocol

## Status

All three required verification commands were run from this staged directory
in WSL Ubuntu with `source ~/.cargo/env` and `export CARGO_BUILD_JOBS=2`.
All three passed with exit code 0. `RUSTFLAGS` was left unset. No Docker, git
operations, credentials, keys, deployment, other staged directory or git
repository was touched.

## Exact commands and exit codes

| # | Exact command | Exit code | Outcome |
| --- | --- | --- | --- |
| 1 | `cargo fmt --all -- --check` | 0 | Clean; no formatting diff. |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | No warnings or errors. |
| 3 | `cargo test --workspace` | 0 | All tests passed. |

The same commands were run together via `bash ./run_checks.sh`, which printed
`FMT_EXIT=0`, `CLIPPY_EXIT=0`, `TEST_EXIT=0`.

Additional command: `cargo run -p conformance --quiet` → exit 0, transcript
below. Test count check: `bash ./count_tests.sh` → `7`, `18`, `20`, total `45`.

## Test pass counts

| Test binary | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| `conformance` unittests (`crates/conformance/src/main.rs`) | 7 | 0 | 0 |
| `protocol-core` unittests (`crates/protocol-core/src/lib.rs`) | 18 | 0 | 0 |
| `provider-transport` unittests (`crates/provider-transport/src/lib.rs`) | 20 | 0 | 0 |
| Doc-tests `protocol_core` | 0 | 0 | 0 |
| Doc-tests `provider_transport` | 0 | 0 | 0 |
| **Total** | **45** | **0** | **0** |

There are **45 `#[test]` functions**, above the required 25.

Representative coverage:

| Test | What it proves |
| --- | --- |
| `version_v1_negotiates` / `unsupported_version_is_rejected` | v1 accepted, all other offered versions rejected. |
| `all_method_variants_have_stable_text_forms` | All five methods have stable, parseable wire tokens. |
| `request_text_roundtrip_is_deterministic` / `response_ok_text_roundtrip` | Compact deterministic serialisation and parsing. |
| `mismatched_response_id_is_rejected` | A response with a foreign id is never accepted. |
| `duplicate_response_for_completed_id_is_rejected` | A completed id cannot produce a second acceptance. |
| `transient_failure_is_retried_then_succeeds` / `transient_failure_can_be_targeted_to_one_request_id` | Bounded retry works and can be scripted at one request. |
| `retry_exhaustion_is_reported` | Exhausted retries are reported, not silently swallowed. |
| `cancel_before_send_prevents_delivery` / `cancel_before_response_is_honoured` | Cancellation blocks send and blocks a pending response. |
| `timeout_is_enforced_with_virtual_clock` / `timeout_after_transient_failure_stops_retry` | Virtual timeout stops work before or after a retry. |
| `errors_never_contain_handles_or_private_payloads` / `audit_and_errors_never_leak_payload_handles` | Handle-shaped payload text never appears in errors or audits. |
| `conformance_report_passes_all_checks` / `approve_precedes_sign_message` | Scripted exchange succeeds and approves before signing. |

## Conformance transcript (exact output)

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

## File table

| Path | Purpose / status |
| --- | --- |
| `Cargo.toml` | Workspace root; pinned metadata; `MIT AND Apache-2.0`. |
| `Cargo.lock` | Generated; only `conformance`, `protocol-core`, `provider-transport`; zero external dependencies. |
| `crates/protocol-core/Cargo.toml` | Manifest for `protocol-core`. |
| `crates/protocol-core/src/lib.rs` | Opaque ids, version negotiation, methods, codec, taxonomy; 18 tests. |
| `crates/provider-transport/Cargo.toml` | Manifest for `provider-transport`; local path dependency only. |
| `crates/provider-transport/src/lib.rs` | `Transport`, in-memory/offline implementations, reliable wrapper, audit log; 20 tests. |
| `crates/conformance/Cargo.toml` | Manifest for `conformance`; local path dependencies only. |
| `crates/conformance/src/main.rs` | Scripted exchange, correlation probe, transcript; 7 tests. |
| `check_fmt.sh` | Helper for the exact fmt command. |
| `check_clippy.sh` | Helper for the exact clippy command. |
| `check_test.sh` | Helper for the exact test command. |
| `run_checks.sh` | Runs the three checks and reports exit codes. |
| `run_conformance.sh` | Prints the conformance transcript. |
| `count_tests.sh` | Counts `#[test]` functions in this workspace. |
| `clean.sh` | Removes only this workspace's `target/`. |
| `README.md` | Documentation, semantics, environment and limitations. |
| `RESULT.md` | This file. |

Every authored `.rs`, `.toml` and `.md` file begins with
`SPDX-License-Identifier: MIT AND Apache-2.0` (comment-prefixed where required).
Shell helpers carry it too.

## Honest limitations / not implemented

- **In-memory model only**; no network, no LEZ, no chain, no sequencer, no
  testnet 0.4, no persistence and no real provider.
- **No real approvals, signatures or keystore.** `approve` and `sign_message`
  return local strings; no cryptography is performed or claimed.
- **No real accounts or users.** Payloads are opaque text and are never
  interpreted; audits contain no payload field.
- **No wall-clock timeout or sleeping.** Backoff/timeout are a deterministic
  virtual clock model only.
- **No CLI beyond the conformance printer, no cross-workspace integration and
  no adoption evidence.**
- **NOT criterion-complete for LP-0021.** This is a local protocol model only.

## Build artifacts

`target/` was removed from this staged directory after verification using
`bash ./clean.sh`. It is not part of the deliverable.
