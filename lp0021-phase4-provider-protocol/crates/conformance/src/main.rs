// SPDX-License-Identifier: MIT AND Apache-2.0
//! Scripted conformance transcript for the LP-0021 local provider protocol.
//!
//! HONESTY: this runs entirely in memory. The "provider" is
//! `InMemoryTransport`, signatures and approvals are local strings, and time
//! is a virtual budget. It demonstrates protocol ordering and correlation —
//! not a real wallet, provider, chain, LEZ call or signature.

#![forbid(unsafe_code)]

use protocol_core::{Method, Payload, ProtocolVersion, Request, RequestId};
use provider_transport::{
    AuditOutcome, InMemoryTransport, ReliableError, ReliableTransport, RetryPolicy,
};

/// Machine-checkable result of one scripted exchange.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConformanceReport {
    pub lines: Vec<String>,
    pub all_four_succeeded: bool,
    pub retry_observed: bool,
    pub ordering_ok: bool,
    pub correlation_ok: bool,
    pub audit_ok: bool,
    pub all_ok: bool,
}

/// Run the scripted exchange: list_accounts → get_balance → approve →
/// sign_message, with one transient failure and one retry, followed by a
/// deliberately mismatched correlation probe.
pub fn run() -> ConformanceReport {
    let mut lines = vec![
        "LP-0021 provider-protocol conformance (in-memory model only)".to_owned(),
        "no LEZ / chain / keystore / real signature / real provider".to_owned(),
    ];

    let mut transport = InMemoryTransport::new();
    // One scripted transient failure on the get_balance request (id 2) only.
    transport.queue_transient_failures_for(RequestId::from_seed(2), 1);
    let policy = RetryPolicy::default();
    let mut wrapper = ReliableTransport::with_policy(transport, policy);

    let steps = [
        (1u64, Method::ListAccounts),
        (2, Method::GetBalance),
        (3, Method::Approve),
        (4, Method::SignMessage),
    ];
    let mut completed_methods = Vec::new();
    let mut all_four_succeeded = true;
    let mut retry_observed = false;

    for (index, (seed, method)) in steps.into_iter().enumerate() {
        let id = RequestId::from_seed(seed);
        let request = Request::new(
            id,
            ProtocolVersion::V1,
            method,
            Payload::new("local-model-only"),
        );
        match wrapper.send(request) {
            Ok(response) => {
                completed_methods.push(method);
                let attempts = wrapper.inner().attempts_for(id);
                if attempts > 1 {
                    retry_observed = true;
                    lines.push(format!(
                        "   transient failure -> retry #{} (backoff {}ms virtual, no sleep)",
                        attempts - 1,
                        policy.backoff_ms(1)
                    ));
                }
                lines.push(format!(
                    "{}. {} req={} -> OK {} attempts={} verified_id={}",
                    index + 1,
                    method,
                    id,
                    response.to_text(),
                    attempts,
                    response.id() == id
                ));
            }
            Err(error) => {
                all_four_succeeded = false;
                lines.push(format!(
                    "{}. {} req={} -> ERROR {}",
                    index + 1,
                    method,
                    id,
                    error
                ));
            }
        }
    }

    let approve_index = completed_methods.iter().position(|m| *m == Method::Approve);
    let sign_index = completed_methods
        .iter()
        .position(|m| *m == Method::SignMessage);
    let ordering_ok = match (approve_index, sign_index) {
        (Some(approve), Some(sign)) => approve < sign,
        _ => false,
    };
    lines.push(format!(
        "Ordering: approve before sign_message = {}",
        yes_no(ordering_ok)
    ));

    // A response for a previously completed id must never be accepted for a
    // different outstanding request.
    let probe_id = RequestId::from_seed(9);
    wrapper
        .inner_mut()
        .force_response_id(RequestId::from_seed(1));
    let probe = Request::new(
        probe_id,
        ProtocolVersion::V1,
        Method::Transfer,
        Payload::new("local-model-only"),
    );
    let correlation_ok = match wrapper.send(probe) {
        Err(ReliableError::CorrelationMismatch) => {
            !wrapper.is_completed(probe_id) && wrapper.inner().attempts_for(probe_id) == 1
        }
        _ => false,
    };
    lines.push(format!(
        "5. correlation probe req={} with response id=r0000000000000001 -> {} (no response accepted without a matching request id)",
        probe_id,
        if correlation_ok { "REJECTED" } else { "ACCEPTED-UNEXPECTEDLY" }
    ));

    lines.push("Audit (method + request id + outcome only):".to_owned());
    for line in wrapper.audit().render() {
        lines.push(format!("  {line}"));
    }

    let audit_lines = wrapper.audit().render().join("\n");
    let entries = wrapper.audit().entries();
    let audit_ok = ordering_ok
        && all_four_succeeded
        && entries
            .iter()
            .filter(|entry| entry.outcome() == AuditOutcome::Success)
            .count()
            == 4
        && entries.iter().any(|entry| {
            entry.outcome() == AuditOutcome::CorrelationMismatch
                && entry.method() == Method::Transfer
        })
        && entries
            .iter()
            .all(|entry| !format!("{entry:?}").contains("SECRET"))
        && audit_lines.contains("|correlation_mismatch")
        && !audit_lines.contains("SECRET");

    let all_ok = all_four_succeeded && retry_observed && ordering_ok && correlation_ok && audit_ok;
    lines.push(format!(
        "Checks: four_successes={} retry={} ordering={} correlation={} audit={}",
        yes_no(all_four_succeeded),
        yes_no(retry_observed),
        yes_no(ordering_ok),
        yes_no(correlation_ok),
        yes_no(audit_ok)
    ));
    lines.push(if all_ok {
        "CONFORMANCE PASSED".to_owned()
    } else {
        "CONFORMANCE FAILED".to_owned()
    });

    ConformanceReport {
        lines,
        all_four_succeeded,
        retry_observed,
        ordering_ok,
        correlation_ok,
        audit_ok,
        all_ok,
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "YES"
    } else {
        "NO"
    }
}

/// Render the transcript with a trailing newline.
pub fn transcript(report: &ConformanceReport) -> String {
    let mut text = report.lines.join("\n");
    text.push('\n');
    text
}

fn main() {
    let report = run();
    print!("{}", transcript(&report));
    if !report.all_ok {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use provider_transport::ReliableError;

    #[test]
    fn conformance_report_passes_all_checks() {
        let report = run();
        assert!(report.all_four_succeeded);
        assert!(report.retry_observed);
        assert!(report.ordering_ok);
        assert!(report.correlation_ok);
        assert!(report.audit_ok);
        assert!(report.all_ok);
    }

    #[test]
    fn approve_precedes_sign_message() {
        let report = run();
        let approve = report
            .lines
            .iter()
            .position(|line| line.contains("3. approve"))
            .unwrap();
        let sign = report
            .lines
            .iter()
            .position(|line| line.contains("4. sign_message"))
            .unwrap();
        assert!(approve < sign);
        assert!(report
            .lines
            .iter()
            .any(|line| line.contains("Ordering: approve before sign_message = YES")));
    }

    #[test]
    fn mismatched_response_is_rejected() {
        let report = run();
        assert!(report
            .lines
            .iter()
            .any(|line| line.contains("correlation probe") && line.contains("REJECTED")));
        assert!(
            run().correlation_ok,
            "correlation probe must reject the mismatched id"
        );
    }

    #[test]
    fn transcript_shows_retry_and_clean_audit() {
        let report = run();
        let text = transcript(&report);
        assert!(text.contains("transient failure -> retry #1"));
        assert!(text.contains("audit|list_accounts|"));
        assert!(text.contains("audit|approve|"));
        assert!(text.contains("|success"));
        assert!(!text.contains("SECRET"));
        assert!(text.ends_with("CONFORMANCE PASSED\n"));
    }

    #[test]
    fn conformance_uses_v1_and_opaque_ids() {
        let report = run();
        assert!(report
            .lines
            .iter()
            .any(|line| line.contains("req=r0000000000000001")));
        assert!(ProtocolVersion::V1.negotiate().is_ok());
    }

    #[test]
    fn audit_lines_have_no_payload_field() {
        let report = run();
        assert!(report.audit_ok);
        for line in transcript(&report)
            .lines()
            .filter(|line| line.starts_with("  audit|"))
        {
            assert_eq!(line.split('|').count(), 4); // "  audit" + 3 fields
            assert!(!line.contains("local-model-only"));
        }
    }

    #[test]
    fn reliable_error_display_never_mentions_handles() {
        let error = ReliableError::CorrelationMismatch;
        assert!(!error.to_string().contains("handle"));
    }
}
