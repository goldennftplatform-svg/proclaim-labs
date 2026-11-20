<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->
# RESULT — LP-0021 Phase 2 local testimonial store model

Date: 2026-11-19

## Status

**LOCAL IN-MEMORY MODEL ONLY.** No chain, no testnet 0.4, no LEZ program, no
deployment, no ZK/verifier, no users, no adoption evidence. **NOT criterion
complete.** Testnet 0.4 is a roadmap target for 30 Nov 2026 and was not
available; no real accounts or testimonials were created.

## Environment

- OS: WSL Ubuntu; workspace path
  `/mnt/c/Users/XPS/Downloads/GFD/staged/lp0021-phase2-testimonials`
- `rustc 1.100.0 (0ef1bc386 2026-11-09)`
- `cargo 1.100.0 (3acc79c0c 2026-11-05)`
- Each command ran after `source ~/.cargo/env` and
  `export CARGO_BUILD_JOBS=2`. No `RUSTFLAGS` were set.
- No keys, tokens, credentials, git repositories, remotes, commits, pushes,
  network authentication, or other staged directories were touched.
- `Cargo.lock` lists exactly three local packages (`adapters`,
  `monthly-report`, `testimonial-store`) and **zero external dependencies**.

## Exact commands and results

All three commands were run from the workspace root. Exit codes below are the
exit codes of the commands themselves.

### 1. Format check

Command: `cargo fmt --all -- --check`

Output: none.

Exit code: **0**.

### 2. Clippy

Command: `cargo clippy --workspace --all-targets -- -D warnings`

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.39s
```

Exit code: **0**; no warnings or errors.

Note: an earlier attempt failed during development (an unused import and a
missing `?Sized` bound in `adapters`). Both were fixed; the output above is the
final authoritative run.

### 3. Tests

Command: `cargo test --workspace`

Verbatim output of the final run (test order varies between runs because the
tests execute in parallel):

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.35s
     Running unittests src/lib.rs (target/debug/build/adapters/8335b41a446c1a05/out/adapters-8335b41a446c1a05)

running 7 tests
test tests::in_memory_sink_stores_and_lists ... ok
test tests::sink_can_be_used_behind_a_trait_object ... ok
test tests::sink_rejects_decreasing_timestamp ... ok
test tests::sink_rejects_duplicate_author ... ok
test tests::sink_reports_its_non_chain_label ... ok
test tests::submit_helper_reports_field_validation_errors ... ok
test tests::unavailable_backend_is_an_explicit_error ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/build/monthly-report/13783d2e7a9eaac3/out/monthly_report-13783d2e7a9eaac3)

running 13 tests
test tests::month_boundary_counts_are_separate ... ok
test tests::counts_records_per_utc_month ... ok
test tests::counts_distinct_authors_per_month ... ok
test tests::monthly_counts_are_empty_by_default ... ok
test tests::threshold_fails_with_fewer_than_150_distinct_authors ... ok
test tests::threshold_fails_with_29_records_in_a_month ... ok
test tests::threshold_fails_with_149_distinct_authors ... ok
test tests::threshold_passes_at_exact_edges ... ok
test tests::threshold_fails_with_only_one_qualifying_month ... ok
test tests::threshold_fails_with_non_consecutive_months ... ok
test tests::threshold_prefers_longest_consecutive_run ... ok
test tests::threshold_result_is_labelled_as_a_local_model ... ok
test tests::threshold_with_zero_records_is_not_met ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/build/testimonial-store/297e5268f50d9047/out/testimonial_store-297e5268f50d9047)

running 30 tests
test tests::empty_submission_id_is_rejected ... ok
test tests::empty_text_is_rejected ... ok
test tests::empty_username_is_rejected ... ok
test tests::fixed_submission_id_is_frozen ... ok
test tests::list_for_submission_filters_by_id ... ok
test tests::decreasing_timestamp_is_rejected ... ok
test tests::duplicate_author_is_rejected ... ok
test tests::list_for_month_with_no_records_is_empty ... ok
test tests::list_for_unknown_submission_is_empty ... ok
test tests::negative_timestamp_is_rejected ... ok
test tests::oversized_text_is_rejected ... ok
test tests::different_authors_are_accepted ... ok
test tests::equal_timestamp_is_accepted ... ok
test tests::record_id_changes_with_text ... ok
test tests::list_for_month_filters_utc_months ... ok
test tests::append_order_is_preserved ... ok
test tests::record_id_is_deterministic ... ok
test tests::store_is_empty_on_creation ... ok
test tests::oversized_username_is_rejected ... ok
test tests::submission_id_parse_rejects_everything_but_the_fixed_id ... ok
test tests::submission_id_unchecked_is_a_fixture_handle_only ... ok
test tests::tampered_record_is_detected ... ok
test tests::text_at_the_length_bound_is_accepted ... ok
test tests::username_at_the_length_bound_is_accepted ... ok
test tests::utc_month_floors_negative_epoch_timestamps ... ok
test tests::utc_month_maps_leap_day_and_year_boundary ... ok
test tests::utc_month_next_crosses_year_boundary ... ok
test tests::valid_testimonial_has_a_valid_record_id ... ok
test tests::whitespace_only_text_is_rejected ... ok
test tests::wrong_submission_id_is_rejected_by_store ... ok

test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests adapters

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests monthly_report

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests testimonial_store

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Exit code: **0**. The 30 store test names above match the `#[test]`
occurrences in `crates/testimonial-store/src/lib.rs`.

### Summary table

| Check | Exact command | Exit code | Outcome |
| --- | --- | --- | --- |
| Format | `cargo fmt --all -- --check` | 0 | Clean; no diff |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | No warnings |
| Tests | `cargo test --workspace` | 0 | 50 passed, 0 failed |

### Test counts

| Test binary | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| `testimonial-store` unittests | 30 | 0 | 0 |
| `monthly-report` unittests | 13 | 0 | 0 |
| `adapters` unittests | 7 | 0 | 0 |
| Doc-tests (3 crates) | 0 | 0 | 0 |
| **Total** | **50** | **0** | **0** |

`#[test]` functions by source file (50 total, exceeding the required 20):

| File | `#[test]` count |
| --- | --- |
| `crates/testimonial-store/src/lib.rs` | 30 |
| `crates/monthly-report/src/lib.rs` | 13 |
| `crates/adapters/src/lib.rs` | 7 |

## Negative cases covered

- Wrong submission id, empty submission id, frozen-id parsing.
- Duplicate `(submission_id, author)` at store and sink level.
- Empty text, whitespace-only text, text over 280 bytes; boundary at 280 bytes.
- Empty username, username over 64 bytes; boundary at 64 bytes.
- Negative timestamp, decreasing timestamp, equal timestamp accepted.
- Tampered record id detection.
- Month-boundary counts, leap day, pre-epoch floor, year rollover.
- Threshold edges: 0, 60, 149, 150 distinct authors; 29 vs 30 records;
  one month, non-consecutive months, exact two-month pass, longest run of
  three with a later two-month run.
- Sink-level unavailable backend and field validation errors.

## Files created

| Path | Purpose |
| --- | --- |
| `Cargo.toml` | Virtual workspace, local path crates only |
| `Cargo.lock` | Generated lockfile: three local packages, zero external deps |
| `README.md` | Honest overview, commands, limitations |
| `RESULT.md` | This file |
| `check_fmt.sh`, `check_clippy.sh`, `check_test.sh`, `run_all.sh`, `clean.sh` | Check/clean helpers |
| `crates/testimonial-store/Cargo.toml`, `crates/testimonial-store/src/lib.rs` | Core model and 30 tests |
| `crates/monthly-report/Cargo.toml`, `crates/monthly-report/src/lib.rs` | Monthly shape model and 13 tests |
| `crates/adapters/Cargo.toml`, `crates/adapters/src/lib.rs` | Sink boundary, in-memory sink, 7 tests |

All authored `.rs`, `.toml`, and `.md` files start with the SPDX identifier
`MIT AND Apache-2.0` in the appropriate comment syntax. `Cargo.lock` is Cargo
generated.

## What is NOT implemented

- **No on-chain program.** No LEZ code, IDL, `.lgx`, sequencer, RPC, program
  deployment, or testnet 0.4 access.
- **No F6 completion.** The real "standard LEZ program that stores each
  testimonial on-chain" does not exist here.
- **No A2/A3 evidence.** The 150-author/30-per-month fixtures are local
  `AuthorId` serials. There are no real users, no distinct on-chain accounts,
  no prior activity checks, no trusted timestamps, and no two-month clock.
- **No ZK/verifier/crypto security.** `RecordId` is deterministic FNV-1a,
  explicitly non-cryptographic; there are no signatures or proofs.
- **No real adapter.** `TestimonialSink` is ready for one, but no LEZ adapter
  exists; `UnavailableSink` reports that honestly.
- **No persistence beyond process memory**, no UI, no SDK integration, and no
  evidence pack.

The `target/` directory was removed after the checks; only source, manifests,
docs, and helper scripts remain.
