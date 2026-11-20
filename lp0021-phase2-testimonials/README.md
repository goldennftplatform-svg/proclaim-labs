<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->
# LP-0021 Phase 2 — local testimonial store model

**Status: LOCAL IN-MEMORY MODEL ONLY.** This is **not** a LEZ program, **not**
on-chain, **not** deployed, **not** testnet 0.4, **not** a ZK verifier, and
**not** criterion complete. It is **not adoption evidence** for A2/A3 and
involves **no real users or accounts**. Testnet 0.4 is a roadmap target for
30 Nov 2026 and is not available here; real users cannot be manufactured.

**Frozen submission ID:** `LP-0021-proclaim-82759d93-58c9-4245-b4bf-6ae8a76d8922`

The purpose of this workspace is to write and test the *semantics* now, so the
same model can be reused as a reference when a real LEZ 0.4 program interface
exists. Nothing here should be presented to judges as satisfying F6 on-chain,
A2, or A3.

## Workspace layout

```
lp0021-phase2-testimonials/
├── Cargo.toml                 # virtual workspace, only local path crates
├── Cargo.lock                 # three local packages, zero external dependencies
├── README.md                  # this file
├── RESULT.md                  # exact commands, outputs, exit codes, limits
├── check_fmt.sh               # cargo fmt --all -- --check
├── check_clippy.sh            # cargo clippy --workspace --all-targets -- -D warnings
├── check_test.sh              # cargo test --workspace
├── run_all.sh                 # runs the three checks
├── clean.sh                   # removes only ./target
└── crates/
    ├── testimonial-store/     # Testimonial, FNV-1a record id, append-only Store
    ├── monthly-report/        # UTC monthly counts + A2/A3 *shape* model
    └── adapters/              # TestimonialSink trait + in-memory implementation
```

## What is modelled

- `Testimonial { submission_id, author, text, username, timestamp_ms,
  record_id }`: custom text, optional username, and the **fixed submission id**
  in every record.
- `Store::append` / `append_record` enforce, in memory:
  - the submission id must equal the frozen id;
  - at most one record per `(submission_id, author)` pair (duplicates rejected);
  - text must be non-empty (whitespace-only counts as empty) and at most
    `MAX_TEXT_BYTES` UTF-8 bytes (280);
  - an optional username must be non-empty when present and at most
    `MAX_USERNAME_BYTES` UTF-8 bytes (64);
  - timestamps must be non-negative and never go backwards relative to the
    previous accepted append;
  - the deterministic record id must still match the record bytes.
- `UtcMonth::from_timestamp_ms` converts epoch milliseconds to a UTC calendar
  month with integer arithmetic; `Store::list_for_submission` and
  `Store::list_for_month` filter the append-only log.
- `monthly-report` counts records and distinct local authors per UTC month and
  returns a structured `ThresholdCheck` for the A2/A3 **shape**: at least 150
  distinct authors overall and at least 30 records in each of at least two
  consecutive months. Every result carries `LOCAL_MODEL_LABEL`, which says it
  is **not adoption evidence**. A passing result is still only a local test
  fixture; no users or on-chain timestamps exist.
- `adapters::TestimonialSink` is the boundary a future real LEZ adapter can
  implement. Today the only implementation is `InMemorySink`; an
  `UnavailableSink` makes the absence of a real backend explicit.

## Honest non-crypto handle

`RecordId` is computed with FNV-1a 64-bit over length-prefixed fields with a
domain separator. It is **deterministic and non-cryptographic**: useful for
reproducible fixtures and detecting accidental mutation inside this process,
but it is **not a signature, not a hash commitment, not a ZK proof, and not a
tamper-proof on-chain guarantee**. `AuthorId` is a caller-supplied opaque
serial, not a wallet, key, or real account.

## Example

```rust
use testimonial_store::{AuthorId, Store, UtcMonth, FIXED_SUBMISSION_ID};

let mut store = Store::new();
let id = store.append(
    FIXED_SUBMISSION_ID,
    AuthorId::new(1),
    "I use wallet XYZ",
    Some("local-fixture"),
    1_767_225_600_000, // 2026-01-01T00:00:00Z, UTC
).unwrap();
assert_eq!(store.len(), 1);
assert_eq!(store.list_for_month(UtcMonth::new(2026, 1).unwrap()).len(), 1);
assert_eq!(store.list()[0].record_id(), id);
```

## Commands and results

From this directory, in WSL with `source ~/.cargo/env` and
`export CARGO_BUILD_JOBS=2`:

| Exact command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | exit 0, no diff |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings |
| `cargo test --workspace` | exit 0, **50 passed, 0 failed** (30 store, 13 monthly-report, 7 adapters; 0 doc-tests) |

See [RESULT.md](RESULT.md) for the verbatim tool output and the test-by-test
result lines. Helper scripts print the same three checks; `bash ./clean.sh`
removes only this workspace's `target/` directory.

## Not implemented / explicitly out of scope

- No LEZ program, `actor_entrypoint!`, IDL, `.lgx` build, sequencer, RPC, or
  transaction submission.
- No chain, no testnet 0.4, no deployment, no on-chain timestamps, no verifier,
  no ZK proofs, no signatures, no real hashing for security.
- No wallet, keys, accounts, authentication, users, recruitment, or Sybil
  resistance. 150 distinct `AuthorId` values in a test are fixtures, not users.
- No persistent database or durable audit log; dropping `Store` drops all data.
- No mini-app UI, SDK integration, real adapter, or evidence pack.
- F6 is **not** complete and A2/A3 are **not** met by this workspace.
