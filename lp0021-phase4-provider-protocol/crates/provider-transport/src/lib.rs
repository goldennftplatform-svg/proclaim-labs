// SPDX-License-Identifier: MIT AND Apache-2.0
//! Correlated, retrying transport wrappers for the LP-0021 local protocol.
//!
//! HONESTY: everything here is an **in-memory model**. `InMemoryTransport`
//! returns scripted local strings; `OfflineTransport` fails loudly. There is
//! no network, no real provider, no chain, no keystore and no real signature.
//! Timeouts and backoff use a deterministic virtual clock: no sleeping and no
//! wall-clock reads. Audits record only method, request id and outcome — never
//! payload contents.

#![forbid(unsafe_code)]

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use protocol_core::{
    Method, Payload, ProtocolError, ProtocolVersion, Request, RequestId, Response,
};

/// Cooperative cancellation flag shared between wrapper and transport model.
#[derive(Debug, Default)]
pub struct Cancellation {
    cancelled: Cell<bool>,
}

impl Cancellation {
    /// Request cancellation from the user side or from a scripted transport.
    pub fn cancel(&self) {
        self.cancelled.set(true);
    }

    /// Whether cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.get()
    }
}

/// The smallest possible provider boundary: one request in, one correlated
/// response out. A transport never sees the audit log or retry policy.
pub trait Transport {
    /// Submit a request. `control` lets a scripted backend model cancellation
    /// while a response is pending.
    fn send(
        &mut self,
        request: &Request,
        control: &Cancellation,
    ) -> Result<Response, ProtocolError>;
}

/// Scripted, in-process provider model. It exists to make request/response
/// handling testable, not to emulate a real provider.
#[derive(Clone, Debug)]
pub struct InMemoryTransport {
    results: BTreeMap<Method, String>,
    transient_failures: usize,
    transient_failures_by_id: BTreeMap<RequestId, usize>,
    forced_response_id: Option<RequestId>,
    cancel_on_attempt: Option<u8>,
    attempts: BTreeMap<RequestId, u8>,
}

impl Default for InMemoryTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryTransport {
    /// Transport with explicit, legible local results for every method.
    pub fn new() -> Self {
        let mut results = BTreeMap::new();
        for method in Method::all() {
            let result = match method {
                Method::ListAccounts => "accounts=0",
                Method::GetBalance => "balance=0",
                Method::Approve => "approval=local-record",
                Method::SignMessage => "sign=local-model-only",
                Method::Transfer => "transfer=local-model-only",
            };
            results.insert(method, result.to_owned());
        }
        Self {
            results,
            transient_failures: 0,
            transient_failures_by_id: BTreeMap::new(),
            forced_response_id: None,
            cancel_on_attempt: None,
            attempts: BTreeMap::new(),
        }
    }

    /// Override the local result for a method. No payload is interpreted.
    pub fn set_result(&mut self, method: Method, result: impl Into<String>) {
        self.results.insert(method, result.into());
    }

    /// Make the next `count` calls fail with the transient `TRANSPORT` error.
    pub fn queue_transient_failures(&mut self, count: usize) {
        self.transient_failures = self.transient_failures.saturating_add(count);
    }

    /// Fail the next `count` attempts of one specific request id. Useful for
    /// scripts that need a retry at a chosen point in an exchange.
    pub fn queue_transient_failures_for(&mut self, id: RequestId, count: usize) {
        let slot = self.transient_failures_by_id.entry(id).or_insert(0);
        *slot = slot.saturating_add(count);
    }

    /// Force the next response to carry a different id (correlation probe).
    pub fn force_response_id(&mut self, id: RequestId) {
        self.forced_response_id = Some(id);
    }

    /// Stop overriding response ids.
    pub fn clear_forced_response_id(&mut self) {
        self.forced_response_id = None;
    }

    /// Model a user cancellation while the `n`-th attempt is in flight. The
    /// response is still produced; the wrapper must refuse to accept it.
    pub fn set_cancel_on_attempt(&mut self, attempt: u8) {
        self.cancel_on_attempt = Some(attempt);
    }

    /// Number of attempts seen for a request id.
    pub fn attempts_for(&self, id: RequestId) -> u8 {
        self.attempts.get(&id).copied().unwrap_or(0)
    }
}

impl Transport for InMemoryTransport {
    fn send(
        &mut self,
        request: &Request,
        control: &Cancellation,
    ) -> Result<Response, ProtocolError> {
        let id = request.id();
        let attempt = self.attempts.entry(id).or_insert(0);
        *attempt = attempt.saturating_add(1);
        let attempt_number = *attempt;

        if self.cancel_on_attempt == Some(attempt_number) {
            control.cancel();
        }
        if control.is_cancelled() {
            return Err(ProtocolError::Cancelled);
        }

        if let Some(remaining) = self.transient_failures_by_id.get_mut(&id) {
            if *remaining > 0 {
                *remaining -= 1;
                return Err(ProtocolError::Transport);
            }
        }
        if self.transient_failures > 0 {
            self.transient_failures -= 1;
            return Err(ProtocolError::Transport);
        }

        let result = self
            .results
            .get(&request.method())
            .cloned()
            .unwrap_or_else(|| format!("{}=local-model-only", request.method().as_str()));
        let response_id = self.forced_response_id.unwrap_or(id);
        Ok(Response::ok(response_id, result))
    }
}

/// Explicit no-backend implementation. It never fabricates a success: every
/// call reports a transport failure that the wrapper can classify and audit.
#[derive(Clone, Copy, Debug, Default)]
pub struct OfflineTransport;

impl Transport for OfflineTransport {
    fn send(
        &mut self,
        _request: &Request,
        _control: &Cancellation,
    ) -> Result<Response, ProtocolError> {
        Err(ProtocolError::Transport)
    }
}

/// Deterministic, bounded retry and virtual-time policy. No real sleeping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: u8,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub attempt_cost_ms: u64,
    pub timeout_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_backoff_ms: 10,
            max_backoff_ms: 40,
            attempt_cost_ms: 1,
            timeout_ms: 1_000,
        }
    }
}

impl RetryPolicy {
    /// Exponential backoff capped at `max_backoff_ms`. The first retry waits
    /// `base_backoff_ms`; later retries double until the cap is reached.
    pub fn backoff_ms(&self, retry_index: u8) -> u64 {
        let mut delay = self.base_backoff_ms;
        for _ in 1..retry_index {
            delay = delay.saturating_mul(2);
        }
        delay.min(self.max_backoff_ms)
    }
}

/// Failures specific to the reliable wrapper. Errors carry no handle, payload
/// or free-form text; the protocol taxonomy remains in [`ProtocolError`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReliableError {
    Protocol(ProtocolError),
    CorrelationMismatch,
    DuplicateResponse,
    Cancelled,
    Timeout,
    RetryExhausted { attempts: u8 },
    AlreadyCompleted,
}

impl Display for ReliableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => write!(f, "protocol error: {error}"),
            Self::CorrelationMismatch => f.write_str("response id does not match request id"),
            Self::DuplicateResponse => f.write_str("duplicate response for completed request"),
            Self::Cancelled => f.write_str("request cancelled before response accepted"),
            Self::Timeout => f.write_str("virtual timeout budget exhausted"),
            Self::RetryExhausted { attempts } => {
                write!(f, "retry budget exhausted after {attempts} attempts")
            }
            Self::AlreadyCompleted => f.write_str("request id already completed"),
        }
    }
}

impl std::error::Error for ReliableError {}

/// One audit line: method, request id, outcome. Nothing else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuditEntry {
    method: Method,
    request_id: RequestId,
    outcome: AuditOutcome,
}

impl AuditEntry {
    /// Audited method.
    pub fn method(&self) -> Method {
        self.method
    }

    /// Audited request id.
    pub fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// Audited outcome.
    pub fn outcome(&self) -> AuditOutcome {
        self.outcome
    }
}

/// Outcome tokens only; no payload or result contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditOutcome {
    Sent,
    TransientFailure,
    RetryScheduled,
    Success,
    CorrelationMismatch,
    DuplicateResponse,
    Cancelled,
    Timeout,
    RetryExhausted,
    Rejected,
}

impl AuditOutcome {
    /// Stable audit token.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sent => "sent",
            Self::TransientFailure => "transient_failure",
            Self::RetryScheduled => "retry_scheduled",
            Self::Success => "success",
            Self::CorrelationMismatch => "correlation_mismatch",
            Self::DuplicateResponse => "duplicate_response",
            Self::Cancelled => "cancelled",
            Self::Timeout => "timeout",
            Self::RetryExhausted => "retry_exhausted",
            Self::Rejected => "rejected",
        }
    }
}

impl Display for AuditOutcome {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Append-only audit log with a deliberately narrow schema.
#[derive(Clone, Debug, Default)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    /// All recorded entries in order.
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Number of recorded entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Render `audit|method|request_id|outcome` lines. No payload field exists.
    pub fn render(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| {
                format!(
                    "audit|{}|{}|{}",
                    entry.method(),
                    entry.request_id(),
                    entry.outcome()
                )
            })
            .collect()
    }
}

/// Correlating, retrying, cancelling and timeout-enforcing wrapper around any
/// [`Transport`]. It is still only a local model: no real time, no I/O.
#[derive(Debug)]
pub struct ReliableTransport<T> {
    inner: T,
    policy: RetryPolicy,
    audit: AuditLog,
    completed: BTreeSet<RequestId>,
    cancelled: BTreeSet<RequestId>,
}

impl<T: Transport> ReliableTransport<T> {
    /// Wrap a transport with the default policy.
    pub fn new(inner: T) -> Self {
        Self::with_policy(inner, RetryPolicy::default())
    }

    /// Wrap a transport with an explicit policy.
    pub fn with_policy(inner: T, policy: RetryPolicy) -> Self {
        Self {
            inner,
            policy,
            audit: AuditLog::default(),
            completed: BTreeSet::new(),
            cancelled: BTreeSet::new(),
        }
    }

    /// Borrow the wrapped transport (used by scripts to inspect attempts).
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Mutably borrow the wrapped transport (used by scripts to inject
    /// scripted failures, response-id overrides or cancellation).
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Borrow the append-only audit log.
    pub fn audit(&self) -> &AuditLog {
        &self.audit
    }

    /// Whether a request id has already produced an accepted response.
    pub fn is_completed(&self, id: RequestId) -> bool {
        self.completed.contains(&id)
    }

    /// Explicit cancellation. The `Cancelled` audit outcome is recorded when
    /// the affected send is refused (a bare id has no method to audit).
    pub fn cancel(&mut self, id: RequestId) -> Result<(), ReliableError> {
        if self.completed.contains(&id) {
            return Err(ReliableError::AlreadyCompleted);
        }
        // No audit line here: a bare id has no method. The Cancelled outcome
        // is recorded when the affected send is refused.
        self.cancelled.insert(id);
        Ok(())
    }

    /// Send one request, returning only a response whose id matches it.
    pub fn send(&mut self, request: Request) -> Result<Response, ReliableError> {
        request.validate().map_err(ReliableError::Protocol)?;
        let id = request.id();
        let method = request.method();
        if self.cancelled.contains(&id) {
            self.record(id, method, AuditOutcome::Cancelled);
            return Err(ReliableError::Cancelled);
        }

        let control = Cancellation::default();
        let max_attempts = self.policy.max_attempts.max(1);
        let mut elapsed = 0u64;

        for attempt in 1..=max_attempts {
            if self.cancelled.contains(&id) || control.is_cancelled() {
                self.record(id, method, AuditOutcome::Cancelled);
                return Err(ReliableError::Cancelled);
            }
            if elapsed.saturating_add(self.policy.attempt_cost_ms) > self.policy.timeout_ms {
                self.record(id, method, AuditOutcome::Timeout);
                return Err(ReliableError::Timeout);
            }
            elapsed = elapsed.saturating_add(self.policy.attempt_cost_ms);
            self.record(id, method, AuditOutcome::Sent);

            match self.inner.send(&request, &control) {
                Ok(response) => {
                    if self.cancelled.contains(&id) || control.is_cancelled() {
                        self.record(id, method, AuditOutcome::Cancelled);
                        return Err(ReliableError::Cancelled);
                    }
                    if response.id() != id {
                        self.record(id, method, AuditOutcome::CorrelationMismatch);
                        return Err(ReliableError::CorrelationMismatch);
                    }
                    if response.validate().is_err() {
                        self.record(id, method, AuditOutcome::Rejected);
                        return Err(ReliableError::Protocol(ProtocolError::InvalidPayload));
                    }
                    if self.completed.contains(&id) {
                        self.record(id, method, AuditOutcome::DuplicateResponse);
                        return Err(ReliableError::DuplicateResponse);
                    }
                    self.completed.insert(id);
                    self.record(id, method, AuditOutcome::Success);
                    return Ok(response);
                }
                Err(error) if error.is_transient() => {
                    self.record(id, method, AuditOutcome::TransientFailure);
                    if attempt == max_attempts {
                        self.record(id, method, AuditOutcome::RetryExhausted);
                        return Err(ReliableError::RetryExhausted {
                            attempts: max_attempts,
                        });
                    }
                    elapsed = elapsed.saturating_add(self.policy.backoff_ms(attempt));
                    if elapsed.saturating_add(self.policy.attempt_cost_ms) > self.policy.timeout_ms
                    {
                        self.record(id, method, AuditOutcome::Timeout);
                        return Err(ReliableError::Timeout);
                    }
                    self.record(id, method, AuditOutcome::RetryScheduled);
                }
                Err(ProtocolError::Cancelled) => {
                    self.record(id, method, AuditOutcome::Cancelled);
                    return Err(ReliableError::Cancelled);
                }
                Err(other) => {
                    self.record(id, method, AuditOutcome::Rejected);
                    return Err(ReliableError::Protocol(other));
                }
            }
        }
        Err(ReliableError::RetryExhausted {
            attempts: max_attempts,
        })
    }

    fn record(&mut self, id: RequestId, method: Method, outcome: AuditOutcome) {
        self.audit.entries.push(AuditEntry {
            method,
            request_id: id,
            outcome,
        });
    }
}

/// Convenience constructor for a `V1` request with an explicit id.
pub fn request(id: RequestId, method: Method, payload: Payload) -> Request {
    Request::new(id, ProtocolVersion::V1, method, payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HANDLE: &str = "pv-handle-sim-SECRET-4242";

    fn req(id: u64, method: Method) -> Request {
        request(RequestId::from_seed(id), method, Payload::EMPTY)
    }

    fn outcomes(wrapper: &ReliableTransport<InMemoryTransport>) -> Vec<AuditOutcome> {
        wrapper
            .audit()
            .entries()
            .iter()
            .map(AuditEntry::outcome)
            .collect()
    }

    #[test]
    fn in_memory_transport_returns_scripted_result() {
        let mut transport = InMemoryTransport::new();
        let response = transport
            .send(&req(1, Method::GetBalance), &Cancellation::default())
            .unwrap();
        assert_eq!(response.id(), RequestId::from_seed(1));
        assert_eq!(response.result(), Some("balance=0"));
        assert_eq!(transport.attempts_for(RequestId::from_seed(1)), 1);
    }

    #[test]
    fn in_memory_transport_can_override_results() {
        let mut transport = InMemoryTransport::new();
        transport.set_result(Method::ListAccounts, "accounts=local");
        let response = transport
            .send(&req(2, Method::ListAccounts), &Cancellation::default())
            .unwrap();
        assert_eq!(response.result(), Some("accounts=local"));
    }

    #[test]
    fn offline_transport_fails_loudly() {
        let mut wrapper = ReliableTransport::with_policy(
            OfflineTransport,
            RetryPolicy {
                max_attempts: 1,
                ..RetryPolicy::default()
            },
        );
        let error = wrapper.send(req(3, Method::ListAccounts)).unwrap_err();
        assert_eq!(error, ReliableError::RetryExhausted { attempts: 1 });
        assert!(!wrapper.is_completed(RequestId::from_seed(3)));
    }

    #[test]
    fn matching_response_is_accepted() {
        let mut wrapper = ReliableTransport::new(InMemoryTransport::new());
        let response = wrapper.send(req(10, Method::ListAccounts)).unwrap();
        assert_eq!(response.id(), RequestId::from_seed(10));
        assert_eq!(response.result(), Some("accounts=0"));
        assert!(wrapper.is_completed(RequestId::from_seed(10)));
        assert!(outcomes(&wrapper).contains(&AuditOutcome::Success));
    }

    #[test]
    fn mismatched_response_id_is_rejected() {
        let mut transport = InMemoryTransport::new();
        transport.force_response_id(RequestId::from_seed(99));
        let mut wrapper = ReliableTransport::new(transport);
        let error = wrapper.send(req(11, Method::GetBalance)).unwrap_err();
        assert_eq!(error, ReliableError::CorrelationMismatch);
        assert!(!wrapper.is_completed(RequestId::from_seed(11)));
        assert!(outcomes(&wrapper).contains(&AuditOutcome::CorrelationMismatch));
    }

    #[test]
    fn duplicate_response_for_completed_id_is_rejected() {
        let transport = InMemoryTransport::new();
        let id = RequestId::from_seed(12);
        let mut wrapper = ReliableTransport::new(transport);
        assert!(wrapper.send(req(12, Method::Approve)).is_ok());
        // Reusing the same correlation id must not yield a second acceptance.
        assert!(wrapper.is_completed(id));
        let error = wrapper.send(req(12, Method::Approve)).unwrap_err();
        assert_eq!(error, ReliableError::DuplicateResponse);
        assert!(outcomes(&wrapper).contains(&AuditOutcome::DuplicateResponse));
    }

    #[test]
    fn transient_failure_is_retried_then_succeeds() {
        let mut transport = InMemoryTransport::new();
        transport.queue_transient_failures(1);
        let mut wrapper = ReliableTransport::new(transport);
        let response = wrapper.send(req(13, Method::GetBalance)).unwrap();
        assert_eq!(response.result(), Some("balance=0"));
        assert_eq!(wrapper.inner().attempts_for(RequestId::from_seed(13)), 2);
        let seen = outcomes(&wrapper);
        assert_eq!(seen.iter().filter(|o| **o == AuditOutcome::Sent).count(), 2);
        assert!(seen.contains(&AuditOutcome::TransientFailure));
        assert!(seen.contains(&AuditOutcome::RetryScheduled));
    }

    #[test]
    fn transient_failure_can_be_targeted_to_one_request_id() {
        let mut transport = InMemoryTransport::new();
        transport.queue_transient_failures_for(RequestId::from_seed(41), 1);
        let mut wrapper = ReliableTransport::new(transport);
        assert!(wrapper
            .send(req(41, Method::Approve))
            .unwrap()
            .result()
            .is_some());
        assert_eq!(wrapper.inner().attempts_for(RequestId::from_seed(41)), 2);
        assert!(wrapper
            .send(req(42, Method::Approve))
            .unwrap()
            .result()
            .is_some());
        assert_eq!(wrapper.inner().attempts_for(RequestId::from_seed(42)), 1);
    }

    #[test]
    fn retry_exhaustion_is_reported() {
        let mut transport = InMemoryTransport::new();
        transport.queue_transient_failures(10);
        let mut wrapper = ReliableTransport::new(transport);
        let error = wrapper.send(req(14, Method::SignMessage)).unwrap_err();
        assert_eq!(error, ReliableError::RetryExhausted { attempts: 3 });
        assert_eq!(wrapper.inner().attempts_for(RequestId::from_seed(14)), 3);
        assert!(outcomes(&wrapper).contains(&AuditOutcome::RetryExhausted));
    }

    #[test]
    fn backoff_is_deterministic_and_bounded() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.backoff_ms(1), 10);
        assert_eq!(policy.backoff_ms(2), 20);
        assert_eq!(policy.backoff_ms(3), 40);
        assert_eq!(policy.backoff_ms(8), 40);
        let zero = RetryPolicy {
            base_backoff_ms: 0,
            ..RetryPolicy::default()
        };
        assert_eq!(zero.backoff_ms(5), 0);
    }

    #[test]
    fn cancel_before_send_prevents_delivery() {
        let id = RequestId::from_seed(15);
        let mut transport = InMemoryTransport::new();
        transport.queue_transient_failures(1);
        let mut wrapper = ReliableTransport::new(transport);
        wrapper.cancel(id).unwrap();
        let error = wrapper.send(req(15, Method::Transfer)).unwrap_err();
        assert_eq!(error, ReliableError::Cancelled);
        assert_eq!(wrapper.inner().attempts_for(id), 0);
        assert!(outcomes(&wrapper).contains(&AuditOutcome::Cancelled));
    }

    #[test]
    fn cancel_before_response_is_honoured() {
        let mut transport = InMemoryTransport::new();
        transport.set_cancel_on_attempt(1);
        let mut wrapper = ReliableTransport::new(transport);
        let error = wrapper.send(req(16, Method::SignMessage)).unwrap_err();
        assert_eq!(error, ReliableError::Cancelled);
        assert!(!wrapper.is_completed(RequestId::from_seed(16)));
        assert!(outcomes(&wrapper).contains(&AuditOutcome::Cancelled));
    }

    #[test]
    fn cancel_after_completion_is_rejected() {
        let id = RequestId::from_seed(17);
        let mut wrapper = ReliableTransport::new(InMemoryTransport::new());
        assert!(wrapper.send(req(17, Method::Approve)).is_ok());
        assert_eq!(wrapper.cancel(id), Err(ReliableError::AlreadyCompleted));
    }

    #[test]
    fn timeout_is_enforced_with_virtual_clock() {
        let policy = RetryPolicy {
            timeout_ms: 1,
            attempt_cost_ms: 2,
            ..RetryPolicy::default()
        };
        let transport = InMemoryTransport::new();
        let mut wrapper = ReliableTransport::with_policy(transport, policy);
        let error = wrapper.send(req(18, Method::ListAccounts)).unwrap_err();
        assert_eq!(error, ReliableError::Timeout);
        assert_eq!(wrapper.inner().attempts_for(RequestId::from_seed(18)), 0);
        assert!(outcomes(&wrapper).contains(&AuditOutcome::Timeout));
    }

    #[test]
    fn timeout_after_transient_failure_stops_retry() {
        let policy = RetryPolicy {
            max_attempts: 5,
            base_backoff_ms: 100,
            max_backoff_ms: 100,
            attempt_cost_ms: 1,
            timeout_ms: 20,
        };
        let mut transport = InMemoryTransport::new();
        transport.queue_transient_failures(1);
        let mut wrapper = ReliableTransport::with_policy(transport, policy);
        let error = wrapper.send(req(19, Method::Transfer)).unwrap_err();
        assert_eq!(error, ReliableError::Timeout);
        assert_eq!(wrapper.inner().attempts_for(RequestId::from_seed(19)), 1);
    }

    #[test]
    fn non_transient_protocol_error_is_not_retried() {
        let mut wrapper = ReliableTransport::new(InMemoryTransport::new());
        let unsupported = Request::new(
            RequestId::from_seed(20),
            ProtocolVersion::from_number(2),
            Method::ListAccounts,
            Payload::EMPTY,
        );
        assert_eq!(
            wrapper.send(unsupported),
            Err(ReliableError::Protocol(ProtocolError::UnsupportedVersion))
        );
        assert_eq!(wrapper.inner().attempts_for(RequestId::from_seed(20)), 0);
    }

    #[test]
    fn audit_records_only_method_id_and_outcome() {
        let mut wrapper = ReliableTransport::new(InMemoryTransport::new());
        assert!(wrapper.send(req(21, Method::ListAccounts)).is_ok());
        for line in wrapper.audit().render() {
            let fields: Vec<&str> = line.split('|').collect();
            assert_eq!(fields.len(), 4);
            assert_eq!(fields[0], "audit");
            assert_eq!(fields[1], "list_accounts");
            assert!(fields[2].starts_with('r'));
        }
    }

    #[test]
    fn audit_and_errors_never_leak_payload_handles() {
        let mut wrapper = ReliableTransport::new(InMemoryTransport::new());
        let payload = Payload::new(format!("owner={HANDLE}"));
        let request = Request::new(
            RequestId::from_seed(22),
            ProtocolVersion::V1,
            Method::SignMessage,
            payload,
        );
        assert!(wrapper.send(request).is_ok());
        for line in wrapper.audit().render() {
            assert!(!line.contains(HANDLE));
        }
        for entry in wrapper.audit().entries() {
            assert!(!format!("{entry:?}").contains(HANDLE));
        }
        let mut transport = InMemoryTransport::new();
        transport.force_response_id(RequestId::from_seed(77));
        let mut mismatched = ReliableTransport::new(transport);
        let request = Request::new(
            RequestId::from_seed(23),
            ProtocolVersion::V1,
            Method::Transfer,
            Payload::new(HANDLE),
        );
        let error = mismatched.send(request).unwrap_err();
        assert!(!error.to_string().contains(HANDLE));
        assert!(!format!("{error:?}").contains(HANDLE));
        for line in mismatched.audit().render() {
            assert!(!line.contains(HANDLE));
        }
    }

    #[test]
    fn policy_max_attempts_is_never_zero() {
        let policy = RetryPolicy {
            max_attempts: 0,
            ..RetryPolicy::default()
        };
        let mut transport = InMemoryTransport::new();
        transport.queue_transient_failures(1);
        let mut wrapper = ReliableTransport::with_policy(transport, policy);
        let error = wrapper.send(req(24, Method::GetBalance)).unwrap_err();
        assert_eq!(error, ReliableError::RetryExhausted { attempts: 1 });
    }

    #[test]
    fn audit_covers_all_four_conformance_methods() {
        let mut wrapper = ReliableTransport::new(InMemoryTransport::new());
        for (index, method) in [
            Method::ListAccounts,
            Method::GetBalance,
            Method::Approve,
            Method::SignMessage,
        ]
        .into_iter()
        .enumerate()
        {
            assert!(wrapper.send(req(30 + index as u64, method)).is_ok());
        }
        let rendered = wrapper.audit().render().join("\n");
        for method in [
            Method::ListAccounts,
            Method::GetBalance,
            Method::Approve,
            Method::SignMessage,
        ] {
            assert!(rendered.contains(method.as_str()));
        }
    }
}
