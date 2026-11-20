// SPDX-License-Identifier: MIT AND Apache-2.0
//! Dependency-free, in-memory model of the semantics required by LP-0021 F6:
//! a program that stores each testimonial with custom text, an optional
//! username, and one fixed submission identifier.
//!
//! # HONESTY / LIMITATIONS
//!
//! * This is **not** a LEZ program, not an on-chain account, not deployed, and
//!   not testnet 0.4. Nothing is submitted anywhere; all state lives in the
//!   current process and disappears when the store is dropped.
//! * [`RecordId`] is produced by a small, deterministic, **non-cryptographic**
//!   FNV-1a 64-bit hash. It is a reproducibility/tamper-evidence handle in
//!   this model only. It is not a signature, not a ZK proof, and offers no
//!   authenticity or integrity guarantee against a determined attacker.
//! * "One record per account" is enforced against an opaque [`AuthorId`]
//!   serial supplied by the caller. No keys, wallets, accounts, users, or
//!   identities are created or verified here.
//! * The monotonic-timestamp rule catches a timestamp older than the previous
//!   append. It is a local rule about our own ordered log, not consensus or a
//!   trusted on-chain clock.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

/// The single submission identifier frozen for this submission. Every record
/// accepted by [`Store`] must carry exactly this value.
pub const FIXED_SUBMISSION_ID: &str = "LP-0021-proclaim-82759d93-58c9-4245-b4bf-6ae8a76d8922";

/// Maximum length of testimonial text, in UTF-8 bytes.
pub const MAX_TEXT_BYTES: usize = 280;

/// Maximum length of an optional username, in UTF-8 bytes.
pub const MAX_USERNAME_BYTES: usize = 64;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const RECORD_DOMAIN: &[u8] = b"lp0021/testimonial-record/v1";

/// Deterministic, **non-cryptographic** FNV-1a over bytes.
fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Length-prefix a field so distinct field splits cannot collide as easily.
fn hash_field(mut hash: u64, label: &[u8], value: &[u8]) -> u64 {
    hash = fnv1a(hash, label);
    hash = fnv1a(hash, &(value.len() as u64).to_le_bytes());
    fnv1a(hash, value)
}

fn compute_record_id(
    submission_id: &str,
    author: AuthorId,
    text: &str,
    username: Option<&str>,
    timestamp_ms: i64,
) -> RecordId {
    let mut hash = fnv1a(FNV_OFFSET, RECORD_DOMAIN);
    hash = hash_field(hash, b"submission_id", submission_id.as_bytes());
    hash = hash_field(hash, b"author", &author.as_u64().to_le_bytes());
    hash = hash_field(hash, b"text", text.as_bytes());
    match username {
        Some(name) => {
            hash = hash_field(hash, b"username-present", &[1u8]);
            hash = hash_field(hash, b"username", name.as_bytes());
        }
        None => {
            hash = hash_field(hash, b"username-present", &[0u8]);
        }
    }
    hash = hash_field(hash, b"timestamp_ms", &timestamp_ms.to_le_bytes());
    RecordId(hash)
}

/// The frozen submission id. The private field means callers cannot silently
/// substitute an id; [`SubmissionId::unchecked`] exists only for negative
/// fixtures, which [`Store`] still rejects.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubmissionId(String);

impl SubmissionId {
    /// The one frozen id used by this submission.
    pub fn fixed() -> Self {
        Self(FIXED_SUBMISSION_ID.to_string())
    }

    /// Accept only the frozen id; anything else is an error.
    pub fn parse(value: &str) -> Result<Self, Error> {
        if value == FIXED_SUBMISSION_ID {
            Ok(Self::fixed())
        } else {
            Err(Error::WrongSubmissionId)
        }
    }

    /// Build an arbitrary id **for fixture and negative-test use only**.
    /// [`Store::append_record`] rejects any id that is not [`SubmissionId::fixed`].
    pub fn unchecked(value: &str) -> Self {
        Self(value.to_string())
    }

    /// The string form of this id.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is the frozen id.
    pub fn is_fixed(&self) -> bool {
        self.0 == FIXED_SUBMISSION_ID
    }
}

impl Display for SubmissionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Opaque stand-in for a distinct account. This is a serial, not a key or a
/// real LEZ account; no user is created by constructing one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthorId(u64);

impl AuthorId {
    /// A deterministic fixture handle.
    pub const fn new(serial: u64) -> Self {
        Self(serial)
    }

    /// The underlying process-local serial.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Display for AuthorId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "acct-{}", self.0)
    }
}

/// Non-cryptographic, deterministic record handle produced by FNV-1a.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordId(u64);

impl RecordId {
    /// The raw 64-bit digest, for reproducible fixtures only.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Display for RecordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "rec-{:016x}", self.0)
    }
}

/// One testimonial record. Fields are private so the fixed-id and record-id
/// invariants can be checked on every append.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Testimonial {
    submission_id: SubmissionId,
    author: AuthorId,
    text: String,
    username: Option<String>,
    timestamp_ms: i64,
    record_id: RecordId,
}

impl Testimonial {
    /// Validate field shapes and compute a deterministic record id.
    ///
    /// The store-level invariants (fixed id, one record per author, monotonic
    /// timestamps) are enforced by [`Store::append_record`].
    pub fn new(
        submission_id: &str,
        author: AuthorId,
        text: &str,
        username: Option<&str>,
        timestamp_ms: i64,
    ) -> Result<Self, Error> {
        if submission_id.is_empty() {
            return Err(Error::EmptySubmissionId);
        }
        if text.trim().is_empty() {
            return Err(Error::EmptyText);
        }
        if text.len() > MAX_TEXT_BYTES {
            return Err(Error::TextTooLong {
                max: MAX_TEXT_BYTES,
            });
        }
        if let Some(name) = username {
            if name.is_empty() {
                return Err(Error::EmptyUsername);
            }
            if name.len() > MAX_USERNAME_BYTES {
                return Err(Error::UsernameTooLong {
                    max: MAX_USERNAME_BYTES,
                });
            }
        }
        if timestamp_ms < 0 {
            return Err(Error::InvalidTimestamp);
        }
        let record_id = compute_record_id(submission_id, author, text, username, timestamp_ms);
        Ok(Self {
            submission_id: SubmissionId::unchecked(submission_id),
            author,
            text: text.to_string(),
            username: username.map(str::to_string),
            timestamp_ms,
            record_id,
        })
    }

    /// The frozen submission identifier carried by this record.
    pub fn submission_id(&self) -> &SubmissionId {
        &self.submission_id
    }

    /// The opaque author handle.
    pub fn author(&self) -> AuthorId {
        self.author
    }

    /// Custom text supplied by the author.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Optional username, if one was supplied.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Unix timestamp in milliseconds.
    pub fn timestamp_ms(&self) -> i64 {
        self.timestamp_ms
    }

    /// Deterministic non-cryptographic record handle.
    pub fn record_id(&self) -> RecordId {
        self.record_id
    }

    /// Recompute the handle from the current fields.
    pub fn expected_record_id(&self) -> RecordId {
        compute_record_id(
            self.submission_id.as_str(),
            self.author,
            &self.text,
            self.username.as_deref(),
            self.timestamp_ms,
        )
    }

    /// Whether the stored handle still matches the record contents.
    pub fn record_id_is_valid(&self) -> bool {
        self.expected_record_id() == self.record_id
    }
}

/// Every rejection reason in the local model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The caller supplied an empty submission id.
    EmptySubmissionId,
    /// The id is not the frozen [`FIXED_SUBMISSION_ID`].
    WrongSubmissionId,
    /// This `(submission_id, author)` pair already has a record.
    DuplicateAuthor,
    /// Text was empty or whitespace-only.
    EmptyText,
    /// Text exceeded the byte limit.
    TextTooLong { max: usize },
    /// A username was supplied as an empty string.
    EmptyUsername,
    /// Username exceeded the byte limit.
    UsernameTooLong { max: usize },
    /// Timestamp predates the Unix epoch.
    InvalidTimestamp,
    /// Timestamp is older than the previous accepted append.
    TimestampWentBackwards { previous_ms: i64, attempted_ms: i64 },
    /// Record handle no longer matches the record bytes.
    RecordIdMismatch,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptySubmissionId => write!(f, "submission id must not be empty"),
            Error::WrongSubmissionId => write!(
                f,
                "submission id must be the frozen id {FIXED_SUBMISSION_ID}"
            ),
            Error::DuplicateAuthor => {
                write!(f, "one testimonial per submission id and author is allowed")
            }
            Error::EmptyText => write!(f, "testimonial text must not be empty"),
            Error::TextTooLong { max } => write!(f, "testimonial text exceeds {max} bytes"),
            Error::EmptyUsername => write!(f, "username must not be an empty string"),
            Error::UsernameTooLong { max } => write!(f, "username exceeds {max} bytes"),
            Error::InvalidTimestamp => write!(f, "timestamp_ms must not be negative"),
            Error::TimestampWentBackwards {
                previous_ms,
                attempted_ms,
            } => write!(
                f,
                "timestamp {attempted_ms} is older than previous append {previous_ms}"
            ),
            Error::RecordIdMismatch => write!(f, "record id does not match record contents"),
        }
    }
}

impl std::error::Error for Error {}

/// A UTC calendar month, derived from a millisecond timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcMonth {
    year: i32,
    month: u8,
}

impl UtcMonth {
    /// Construct a month; months outside 1..=12 are rejected.
    pub const fn new(year: i32, month: u8) -> Option<Self> {
        if month == 0 || month > 12 {
            None
        } else {
            Some(Self { year, month })
        }
    }

    /// Convert a Unix millisecond timestamp to its UTC calendar month using
    /// integer arithmetic (days-from-civil inverse; no chrono dependency).
    pub fn from_timestamp_ms(timestamp_ms: i64) -> Self {
        let days = timestamp_ms.div_euclid(86_400_000);
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let month = mp + if mp < 10 { 3 } else { -9 };
        let year = y + if month <= 2 { 1 } else { 0 };
        Self {
            year: year as i32,
            month: month as u8,
        }
    }

    /// Calendar year.
    pub const fn year(self) -> i32 {
        self.year
    }

    /// Calendar month in 1..=12.
    pub const fn month(self) -> u8 {
        self.month
    }

    /// The following calendar month.
    pub fn next(self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }
}

impl Display for UtcMonth {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

/// Append-only, in-memory store with the F6 invariants.
#[derive(Clone, Debug, Default)]
pub struct Store {
    records: Vec<Testimonial>,
    seen: BTreeMap<(String, AuthorId), RecordId>,
    last_timestamp_ms: Option<i64>,
}

impl Store {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a record and append it in one step.
    pub fn append(
        &mut self,
        submission_id: &str,
        author: AuthorId,
        text: &str,
        username: Option<&str>,
        timestamp_ms: i64,
    ) -> Result<RecordId, Error> {
        let record = Testimonial::new(submission_id, author, text, username, timestamp_ms)?;
        self.append_record(record)
    }

    /// Append a pre-built record after checking every store-level invariant.
    pub fn append_record(&mut self, record: Testimonial) -> Result<RecordId, Error> {
        if !record.submission_id().is_fixed() {
            return Err(Error::WrongSubmissionId);
        }
        let key = (record.submission_id().as_str().to_string(), record.author());
        if self.seen.contains_key(&key) {
            return Err(Error::DuplicateAuthor);
        }
        if let Some(previous_ms) = self.last_timestamp_ms {
            if record.timestamp_ms() < previous_ms {
                return Err(Error::TimestampWentBackwards {
                    previous_ms,
                    attempted_ms: record.timestamp_ms(),
                });
            }
        }
        if !record.record_id_is_valid() {
            return Err(Error::RecordIdMismatch);
        }
        let id = record.record_id();
        self.seen.insert(key, id);
        self.last_timestamp_ms = Some(record.timestamp_ms());
        self.records.push(record);
        Ok(id)
    }

    /// Number of accepted records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether no records have been accepted.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Timestamp of the most recent accepted append.
    pub fn last_timestamp_ms(&self) -> Option<i64> {
        self.last_timestamp_ms
    }

    /// All accepted records, in append order.
    pub fn list(&self) -> Vec<Testimonial> {
        self.records.clone()
    }

    /// Records whose submission id equals `submission_id`.
    pub fn list_for_submission(&self, submission_id: &str) -> Vec<Testimonial> {
        self.records
            .iter()
            .filter(|record| record.submission_id().as_str() == submission_id)
            .cloned()
            .collect()
    }

    /// Records whose UTC calendar month equals `month`.
    pub fn list_for_month(&self, month: UtcMonth) -> Vec<Testimonial> {
        self.records
            .iter()
            .filter(|record| UtcMonth::from_timestamp_ms(record.timestamp_ms()) == month)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JAN_2026: i64 = 1_767_225_600_000;
    const DEC_2025_LAST_MS: i64 = 1_767_225_599_999;
    const FEB_2026: i64 = 1_769_904_000_000;

    fn record(author: u64, text: &str, username: Option<&str>, ms: i64) -> Testimonial {
        Testimonial::new(
            FIXED_SUBMISSION_ID,
            AuthorId::new(author),
            text,
            username,
            ms,
        )
        .unwrap()
    }

    fn append(store: &mut Store, author: u64, ms: i64) -> RecordId {
        store
            .append(
                FIXED_SUBMISSION_ID,
                AuthorId::new(author),
                "I use wallet XYZ",
                None,
                ms,
            )
            .unwrap()
    }

    #[test]
    fn fixed_submission_id_is_frozen() {
        assert_eq!(
            FIXED_SUBMISSION_ID,
            "LP-0021-proclaim-82759d93-58c9-4245-b4bf-6ae8a76d8922"
        );
        assert!(SubmissionId::fixed().is_fixed());
        assert_eq!(SubmissionId::fixed().to_string(), FIXED_SUBMISSION_ID);
    }

    #[test]
    fn submission_id_parse_rejects_everything_but_the_fixed_id() {
        assert!(SubmissionId::parse(FIXED_SUBMISSION_ID).is_ok());
        assert_eq!(
            SubmissionId::parse("LP-0021-proclaim-not-the-frozen-id"),
            Err(Error::WrongSubmissionId)
        );
        assert_eq!(SubmissionId::parse(""), Err(Error::WrongSubmissionId));
    }

    #[test]
    fn submission_id_unchecked_is_a_fixture_handle_only() {
        let fixture = SubmissionId::unchecked("not-frozen");
        assert!(!fixture.is_fixed());
        assert_eq!(fixture.to_string(), "not-frozen");
    }

    #[test]
    fn record_id_is_deterministic() {
        let a = record(1, "custom text", Some("alice"), JAN_2026);
        let b = record(1, "custom text", Some("alice"), JAN_2026);
        assert_eq!(a.record_id(), b.record_id());
        assert_eq!(a.record_id().as_u64(), b.expected_record_id().as_u64());
    }

    #[test]
    fn record_id_changes_with_text() {
        let a = record(1, "first", None, JAN_2026);
        let b = record(1, "second", None, JAN_2026);
        assert_ne!(a.record_id(), b.record_id());
    }

    #[test]
    fn valid_testimonial_has_a_valid_record_id() {
        let item = record(3, "I use wallet XYZ", None, JAN_2026);
        assert!(item.record_id_is_valid());
        assert_eq!(item.submission_id().as_str(), FIXED_SUBMISSION_ID);
        assert_eq!(item.username(), None);
    }

    #[test]
    fn empty_submission_id_is_rejected() {
        let result = Testimonial::new("", AuthorId::new(1), "text", None, JAN_2026);
        assert_eq!(result.err(), Some(Error::EmptySubmissionId));
    }

    #[test]
    fn wrong_submission_id_is_rejected_by_store() {
        let wrong = SubmissionId::unchecked("LP-0021-proclaim-wrong");
        let item =
            Testimonial::new(wrong.as_str(), AuthorId::new(1), "text", None, JAN_2026).unwrap();
        let mut store = Store::new();
        assert_eq!(store.append_record(item), Err(Error::WrongSubmissionId));
        assert!(store.is_empty());
    }

    #[test]
    fn empty_text_is_rejected() {
        let result = Testimonial::new(FIXED_SUBMISSION_ID, AuthorId::new(1), "", None, JAN_2026);
        assert_eq!(result.err(), Some(Error::EmptyText));
    }

    #[test]
    fn whitespace_only_text_is_rejected() {
        let result = Testimonial::new(
            FIXED_SUBMISSION_ID,
            AuthorId::new(1),
            "   \t\n ",
            None,
            JAN_2026,
        );
        assert_eq!(result.err(), Some(Error::EmptyText));
    }

    #[test]
    fn oversized_text_is_rejected() {
        let too_long = "x".repeat(MAX_TEXT_BYTES + 1);
        let result = Testimonial::new(
            FIXED_SUBMISSION_ID,
            AuthorId::new(1),
            &too_long,
            None,
            JAN_2026,
        );
        assert_eq!(
            result.err(),
            Some(Error::TextTooLong {
                max: MAX_TEXT_BYTES
            })
        );
    }

    #[test]
    fn text_at_the_length_bound_is_accepted() {
        let bounded = "x".repeat(MAX_TEXT_BYTES);
        let item = record(1, &bounded, None, JAN_2026);
        assert_eq!(item.text().len(), MAX_TEXT_BYTES);
        assert!(item.record_id_is_valid());
    }

    #[test]
    fn empty_username_is_rejected() {
        let result = Testimonial::new(
            FIXED_SUBMISSION_ID,
            AuthorId::new(1),
            "text",
            Some(""),
            JAN_2026,
        );
        assert_eq!(result.err(), Some(Error::EmptyUsername));
    }

    #[test]
    fn oversized_username_is_rejected() {
        let too_long = "u".repeat(MAX_USERNAME_BYTES + 1);
        let result = Testimonial::new(
            FIXED_SUBMISSION_ID,
            AuthorId::new(1),
            "text",
            Some(&too_long),
            JAN_2026,
        );
        assert_eq!(
            result.err(),
            Some(Error::UsernameTooLong {
                max: MAX_USERNAME_BYTES
            })
        );
    }

    #[test]
    fn username_at_the_length_bound_is_accepted() {
        let bounded = "u".repeat(MAX_USERNAME_BYTES);
        let item = record(1, "text", Some(&bounded), JAN_2026);
        assert_eq!(item.username(), Some(bounded.as_str()));
    }

    #[test]
    fn negative_timestamp_is_rejected() {
        let result = Testimonial::new(FIXED_SUBMISSION_ID, AuthorId::new(1), "text", None, -1);
        assert_eq!(result.err(), Some(Error::InvalidTimestamp));
    }

    #[test]
    fn decreasing_timestamp_is_rejected() {
        let mut store = Store::new();
        append(&mut store, 1, JAN_2026);
        let earlier = record(2, "older append", None, JAN_2026 - 1);
        assert_eq!(
            store.append_record(earlier),
            Err(Error::TimestampWentBackwards {
                previous_ms: JAN_2026,
                attempted_ms: JAN_2026 - 1,
            })
        );
        assert_eq!(store.len(), 1);
        assert_eq!(store.last_timestamp_ms(), Some(JAN_2026));
    }

    #[test]
    fn equal_timestamp_is_accepted() {
        let mut store = Store::new();
        append(&mut store, 1, JAN_2026);
        append(&mut store, 2, JAN_2026);
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn duplicate_author_is_rejected() {
        let mut store = Store::new();
        append(&mut store, 7, JAN_2026);
        let duplicate = record(7, "second try", None, JAN_2026 + 1);
        assert_eq!(store.append_record(duplicate), Err(Error::DuplicateAuthor));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn different_authors_are_accepted() {
        let mut store = Store::new();
        append(&mut store, 1, JAN_2026);
        append(&mut store, 2, JAN_2026);
        assert_eq!(store.len(), 2);
        assert_ne!(store.list()[0].author(), store.list()[1].author());
    }

    #[test]
    fn append_order_is_preserved() {
        let mut store = Store::new();
        append(&mut store, 11, JAN_2026);
        append(&mut store, 22, JAN_2026 + 1);
        append(&mut store, 33, FEB_2026);
        let authors: Vec<u64> = store
            .list()
            .iter()
            .map(|item| item.author().as_u64())
            .collect();
        assert_eq!(authors, vec![11, 22, 33]);
    }

    #[test]
    fn list_for_submission_filters_by_id() {
        let mut store = Store::new();
        append(&mut store, 1, JAN_2026);
        assert_eq!(store.list_for_submission(FIXED_SUBMISSION_ID).len(), 1);
        assert_eq!(
            store.list_for_submission(FIXED_SUBMISSION_ID)[0].author(),
            AuthorId::new(1)
        );
    }

    #[test]
    fn list_for_unknown_submission_is_empty() {
        let mut store = Store::new();
        append(&mut store, 1, JAN_2026);
        assert!(store
            .list_for_submission("some-other-submission")
            .is_empty());
    }

    #[test]
    fn list_for_month_filters_utc_months() {
        let mut store = Store::new();
        append(&mut store, 1, DEC_2025_LAST_MS);
        append(&mut store, 2, JAN_2026);
        let january = UtcMonth::new(2026, 1).unwrap();
        assert_eq!(store.list_for_month(january).len(), 1);
        assert_eq!(store.list_for_month(january)[0].author(), AuthorId::new(2));
    }

    #[test]
    fn list_for_month_with_no_records_is_empty() {
        let mut store = Store::new();
        append(&mut store, 1, JAN_2026);
        assert!(store
            .list_for_month(UtcMonth::new(2025, 12).unwrap())
            .is_empty());
    }

    #[test]
    fn utc_month_maps_leap_day_and_year_boundary() {
        assert_eq!(
            UtcMonth::from_timestamp_ms(DEC_2025_LAST_MS).to_string(),
            "2025-12"
        );
        assert_eq!(UtcMonth::from_timestamp_ms(JAN_2026).to_string(), "2026-01");
        assert_eq!(
            UtcMonth::from_timestamp_ms(1_709_208_000_000).to_string(),
            "2024-02"
        );
    }

    #[test]
    fn utc_month_floors_negative_epoch_timestamps() {
        assert_eq!(UtcMonth::from_timestamp_ms(-1).to_string(), "1969-12");
    }

    #[test]
    fn utc_month_next_crosses_year_boundary() {
        assert_eq!(
            UtcMonth::new(2025, 12).unwrap().next(),
            UtcMonth::new(2026, 1).unwrap()
        );
        assert_eq!(
            UtcMonth::new(2026, 1).unwrap().next(),
            UtcMonth::new(2026, 2).unwrap()
        );
        assert!(UtcMonth::new(2026, 13).is_none());
        assert!(UtcMonth::new(2026, 0).is_none());
    }

    #[test]
    fn tampered_record_is_detected() {
        let mut store = Store::new();
        let mut item = record(7, "original text", None, JAN_2026);
        assert!(item.record_id_is_valid());
        item.text = "tampered text".to_string();
        assert!(!item.record_id_is_valid());
        assert_eq!(store.append_record(item), Err(Error::RecordIdMismatch));
        assert!(store.is_empty());
    }

    #[test]
    fn store_is_empty_on_creation() {
        let store = Store::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert_eq!(store.last_timestamp_ms(), None);
        assert!(store.list().is_empty());
    }
}
