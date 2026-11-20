// SPDX-License-Identifier: MIT AND Apache-2.0
//! Sink boundary between the local testimonial core and a future real LEZ
//! on-chain adapter.
//!
//! Core logic depends only on [`TestimonialSink`], so adding a real adapter
//! later does not require changes to record construction or validation. The
//! only implementation here is [`InMemorySink`].
//!
//! # HONESTY / LIMITATIONS
//!
//! There is no LEZ adapter, no RPC, no transaction, no IDL, no ZK verifier,
//! and no deployment in this workspace. [`InMemorySink`] is a process-local
//! wrapper around the local model store.

#![forbid(unsafe_code)]

use testimonial_store::{AuthorId, Error as StoreError, RecordId, Store, Testimonial};

/// Errors produced by a sink implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SinkError {
    /// The core store rejected the record.
    Core(StoreError),
    /// The backend is not available in this local build.
    BackendUnavailable(&'static str),
}

impl std::fmt::Display for SinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SinkError::Core(error) => write!(f, "testimonial core rejected the record: {error}"),
            SinkError::BackendUnavailable(name) => {
                write!(f, "backend {name} is not available in this local model")
            }
        }
    }
}

impl std::error::Error for SinkError {}

/// Result alias for sink operations.
pub type Result<T> = std::result::Result<T, SinkError>;

/// A place that can accept one testimonial record and list what it holds.
///
/// A future `LezProgramSink` can implement this trait by translating a record
/// into a program instruction, without touching the core validation rules.
pub trait TestimonialSink {
    /// Accept one already-validated-shape record.
    fn submit(&mut self, record: Testimonial) -> Result<RecordId>;

    /// List the records this sink currently holds.
    fn records(&self) -> Vec<Testimonial>;

    /// Honest label describing the backing implementation.
    fn backend_label(&self) -> &'static str;
}

/// The only sink available here: a wrapper around the in-memory model store.
#[derive(Clone, Debug, Default)]
pub struct InMemorySink {
    store: Store,
}

impl InMemorySink {
    /// An empty in-memory sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Read-only access to the underlying local store.
    pub fn store(&self) -> &Store {
        &self.store
    }
}

impl TestimonialSink for InMemorySink {
    fn submit(&mut self, record: Testimonial) -> Result<RecordId> {
        self.store.append_record(record).map_err(SinkError::Core)
    }

    fn records(&self) -> Vec<Testimonial> {
        self.store.list()
    }

    fn backend_label(&self) -> &'static str {
        "in-memory local model - NOT a LEZ chain or testnet"
    }
}

/// Convenience helper mirroring [`Store::append`] through any sink.
pub fn submit_testimonial<S: TestimonialSink + ?Sized>(
    sink: &mut S,
    submission_id: &str,
    author: AuthorId,
    text: &str,
    username: Option<&str>,
    timestamp_ms: i64,
) -> Result<RecordId> {
    let record = Testimonial::new(submission_id, author, text, username, timestamp_ms)
        .map_err(SinkError::Core)?;
    sink.submit(record)
}

/// A sink that is explicitly unavailable, for testing the boundary error.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableSink;

impl TestimonialSink for UnavailableSink {
    fn submit(&mut self, _record: Testimonial) -> Result<RecordId> {
        Err(SinkError::BackendUnavailable("LEZ testnet 0.4"))
    }

    fn records(&self) -> Vec<Testimonial> {
        Vec::new()
    }

    fn backend_label(&self) -> &'static str {
        "unavailable - no LEZ program exists in this local model"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testimonial_store::{Error, FIXED_SUBMISSION_ID};

    const JAN_2026: i64 = 1_767_225_600_000;

    #[test]
    fn in_memory_sink_stores_and_lists() {
        let mut sink = InMemorySink::new();
        let id = submit_testimonial(
            &mut sink,
            FIXED_SUBMISSION_ID,
            AuthorId::new(1),
            "I use wallet XYZ",
            None,
            JAN_2026,
        )
        .unwrap();
        assert_eq!(sink.records().len(), 1);
        assert_eq!(sink.records()[0].record_id(), id);
        assert_eq!(sink.store().len(), 1);
    }

    #[test]
    fn sink_reports_its_non_chain_label() {
        let sink = InMemorySink::new();
        assert!(sink.backend_label().contains("in-memory"));
        assert!(sink.backend_label().contains("NOT a LEZ chain"));
        assert_eq!(
            UnavailableSink.backend_label(),
            "unavailable - no LEZ program exists in this local model"
        );
    }

    #[test]
    fn sink_rejects_duplicate_author() {
        let mut sink = InMemorySink::new();
        submit_testimonial(
            &mut sink,
            FIXED_SUBMISSION_ID,
            AuthorId::new(9),
            "first",
            None,
            JAN_2026,
        )
        .unwrap();
        let result = submit_testimonial(
            &mut sink,
            FIXED_SUBMISSION_ID,
            AuthorId::new(9),
            "second",
            None,
            JAN_2026 + 1,
        );
        assert_eq!(result, Err(SinkError::Core(Error::DuplicateAuthor)));
        assert_eq!(sink.records().len(), 1);
    }

    #[test]
    fn sink_rejects_decreasing_timestamp() {
        let mut sink = InMemorySink::new();
        submit_testimonial(
            &mut sink,
            FIXED_SUBMISSION_ID,
            AuthorId::new(1),
            "first",
            None,
            JAN_2026 + 10,
        )
        .unwrap();
        let result = submit_testimonial(
            &mut sink,
            FIXED_SUBMISSION_ID,
            AuthorId::new(2),
            "earlier",
            None,
            JAN_2026,
        );
        assert!(matches!(
            result,
            Err(SinkError::Core(Error::TimestampWentBackwards { .. }))
        ));
        assert_eq!(sink.records().len(), 1);
    }

    #[test]
    fn sink_can_be_used_behind_a_trait_object() {
        let mut sink: Box<dyn TestimonialSink> = Box::new(InMemorySink::new());
        submit_testimonial(
            sink.as_mut(),
            FIXED_SUBMISSION_ID,
            AuthorId::new(4),
            "through the trait",
            Some("local-user"),
            JAN_2026,
        )
        .unwrap();
        assert_eq!(sink.records().len(), 1);
        assert!(sink.backend_label().contains("NOT a LEZ chain"));
    }

    #[test]
    fn submit_helper_reports_field_validation_errors() {
        let mut sink = InMemorySink::new();
        let result = submit_testimonial(
            &mut sink,
            FIXED_SUBMISSION_ID,
            AuthorId::new(1),
            "",
            None,
            JAN_2026,
        );
        assert_eq!(result, Err(SinkError::Core(Error::EmptyText)));
        assert!(sink.records().is_empty());
    }

    #[test]
    fn unavailable_backend_is_an_explicit_error() {
        let mut sink = UnavailableSink;
        let result = submit_testimonial(
            &mut sink,
            FIXED_SUBMISSION_ID,
            AuthorId::new(1),
            "text",
            None,
            JAN_2026,
        );
        assert_eq!(
            result,
            Err(SinkError::BackendUnavailable("LEZ testnet 0.4"))
        );
        assert!(result.unwrap_err().to_string().contains("not available"));
    }
}
