// SPDX-License-Identifier: MIT AND Apache-2.0
//! Versioned, correlated request/response envelopes for the LP-0021 local
//! provider-protocol model.
//!
//! HONESTY: this is a pure, dependency-free text protocol for an **in-memory**
//! model. There is no network, no real provider, no LEZ, no chain, no keystore
//! and no real signature. Payloads are opaque strings: this crate serialises
//! them but never interprets, authenticates or logs them. Errors are fixed
//! variants with no free-form fields, so they cannot carry account handles or
//! other private data.

#![forbid(unsafe_code)]

use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

/// Process-local generator for fresh opaque request identifiers.
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

const REQUEST_TAG: &str = "REQ";
const RESPONSE_TAG: &str = "RES";
const OK_TAG: &str = "OK";
const ERR_TAG: &str = "ERR";

/// An opaque correlation identifier. Its numeric representation is private;
/// callers can display or parse it but cannot inspect or construct one by
/// guessing a handle-like value. `from_seed` exists only to make local tests
/// and the conformance transcript deterministic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(u64);

impl RequestId {
    /// Allocate a fresh process-local identifier.
    pub fn new() -> Self {
        Self(NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Deterministic identifier for scripts and tests. Not a secret.
    pub fn from_seed(seed: u64) -> Self {
        Self(seed)
    }

    /// Canonical opaque text form, e.g. `r0000000000000001`.
    pub fn to_text(self) -> String {
        format!("r{:016x}", self.0)
    }

    /// Parse a canonical opaque text form.
    pub fn from_text(text: &str) -> Result<Self, ProtocolError> {
        let digits = text
            .strip_prefix('r')
            .ok_or(ProtocolError::InvalidPayload)?;
        if digits.is_empty() || digits.len() > 16 {
            return Err(ProtocolError::InvalidPayload);
        }
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ProtocolError::InvalidPayload);
        }
        u64::from_str_radix(digits, 16)
            .map(Self)
            .map_err(|_| ProtocolError::InvalidPayload)
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for RequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text())
    }
}

/// Protocol version. Only `V1` is supported; any other number is offered by
/// callers for negotiation and rejected explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolVersion(u16);

impl ProtocolVersion {
    /// The only supported version.
    pub const V1: Self = Self(1);

    /// Represent an offered version number, supported or not.
    pub fn from_number(number: u16) -> Self {
        Self(number)
    }

    /// The offered version number (never a handle or other private value).
    pub fn number(self) -> u16 {
        self.0
    }

    /// Negotiate against the locally supported version set.
    pub fn negotiate(self) -> Result<Self, ProtocolError> {
        if self == Self::V1 {
            Ok(Self::V1)
        } else {
            Err(ProtocolError::UnsupportedVersion)
        }
    }

    /// Canonical text form, e.g. `v1`.
    pub fn to_text(self) -> String {
        format!("v{}", self.0)
    }

    /// Parse a version token. Unsupported but well-formed versions parse
    /// successfully so that [`Self::negotiate`] can reject them explicitly.
    pub fn from_text(text: &str) -> Result<Self, ProtocolError> {
        let digits = text
            .strip_prefix('v')
            .ok_or(ProtocolError::InvalidPayload)?;
        if digits.is_empty() {
            return Err(ProtocolError::InvalidPayload);
        }
        digits
            .parse::<u16>()
            .map(Self)
            .map_err(|_| ProtocolError::InvalidPayload)
    }
}

impl Display for ProtocolVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text())
    }
}

/// Methods understood by the local protocol. The list is deliberately small
/// and explicit: an unknown method name is an error, never a fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Method {
    ListAccounts,
    GetBalance,
    Approve,
    SignMessage,
    Transfer,
}

impl Method {
    /// Stable wire token.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ListAccounts => "list_accounts",
            Self::GetBalance => "get_balance",
            Self::Approve => "approve",
            Self::SignMessage => "sign_message",
            Self::Transfer => "transfer",
        }
    }

    /// Parse a wire token.
    pub fn from_text(text: &str) -> Result<Self, ProtocolError> {
        match text {
            "list_accounts" => Ok(Self::ListAccounts),
            "get_balance" => Ok(Self::GetBalance),
            "approve" => Ok(Self::Approve),
            "sign_message" => Ok(Self::SignMessage),
            "transfer" => Ok(Self::Transfer),
            _ => Err(ProtocolError::UnknownMethod),
        }
    }

    /// All known methods, in wire-order; useful for exhaustive local checks.
    pub fn all() -> [Self; 5] {
        [
            Self::ListAccounts,
            Self::GetBalance,
            Self::Approve,
            Self::SignMessage,
            Self::Transfer,
        ]
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Fixed, information-light protocol error taxonomy.
///
/// No variant carries a string or account handle, so neither `Display` nor
/// `Debug` output can disclose private data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProtocolError {
    UnsupportedVersion,
    UnknownMethod,
    Timeout,
    Cancelled,
    Denied,
    Transport,
    InvalidPayload,
}

impl ProtocolError {
    /// Stable error token used in serialised responses.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "UNSUPPORTED_VERSION",
            Self::UnknownMethod => "UNKNOWN_METHOD",
            Self::Timeout => "TIMEOUT",
            Self::Cancelled => "CANCELLED",
            Self::Denied => "DENIED",
            Self::Transport => "TRANSPORT",
            Self::InvalidPayload => "INVALID_PAYLOAD",
        }
    }

    /// Parse an error token.
    pub fn from_text(text: &str) -> Result<Self, ProtocolError> {
        match text {
            "UNSUPPORTED_VERSION" => Ok(Self::UnsupportedVersion),
            "UNKNOWN_METHOD" => Ok(Self::UnknownMethod),
            "TIMEOUT" => Ok(Self::Timeout),
            "CANCELLED" => Ok(Self::Cancelled),
            "DENIED" => Ok(Self::Denied),
            "TRANSPORT" => Ok(Self::Transport),
            "INVALID_PAYLOAD" => Ok(Self::InvalidPayload),
            _ => Err(Self::InvalidPayload),
        }
    }

    /// True for failures a reliable wrapper may safely retry. Cancellation is
    /// deliberately not transient: it is an explicit user decision.
    pub fn is_transient(self) -> bool {
        matches!(self, Self::Transport | Self::Timeout)
    }

    /// All taxonomy variants, for exhaustive local checks.
    pub fn all() -> [Self; 7] {
        [
            Self::UnsupportedVersion,
            Self::UnknownMethod,
            Self::Timeout,
            Self::Cancelled,
            Self::Denied,
            Self::Transport,
            Self::InvalidPayload,
        ]
    }
}

impl Display for ProtocolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for ProtocolError {}

/// Opaque request body. The protocol carries it verbatim and never interprets
/// it; callers must not put secrets in it in a real system.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Payload(String);

impl Payload {
    /// Empty payload.
    pub const EMPTY: Self = Self(String::new());

    /// Wrap a text payload.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Borrow the opaque payload text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the payload is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for Payload {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A versioned, correlated request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    id: RequestId,
    version: ProtocolVersion,
    method: Method,
    payload: Payload,
}

impl Request {
    /// Construct a request from explicitly chosen parts.
    pub fn new(id: RequestId, version: ProtocolVersion, method: Method, payload: Payload) -> Self {
        Self {
            id,
            version,
            method,
            payload,
        }
    }

    /// Construct a fresh `V1` request.
    pub fn fresh_v1(method: Method, payload: Payload) -> Self {
        Self::new(RequestId::new(), ProtocolVersion::V1, method, payload)
    }

    /// Correlation identifier.
    pub fn id(&self) -> RequestId {
        self.id
    }

    /// Offered protocol version.
    pub fn version(&self) -> ProtocolVersion {
        self.version
    }

    /// Requested method.
    pub fn method(&self) -> Method {
        self.method
    }

    /// Opaque payload.
    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    /// Reject unsupported versions before any work is attempted.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.version.negotiate().map(|_| ())
    }

    /// Canonical compact text form: `REQ|id|version|method|payload`.
    pub fn to_text(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            REQUEST_TAG,
            self.id,
            self.version,
            self.method,
            escape(self.payload.as_str())
        )
    }

    /// Parse a canonical request text form.
    pub fn from_text(text: &str) -> Result<Self, ProtocolError> {
        let fields = split_fields(text)?;
        if fields.len() != 5 || fields[0] != REQUEST_TAG {
            return Err(ProtocolError::InvalidPayload);
        }
        Ok(Self {
            id: RequestId::from_text(&fields[1])?,
            version: ProtocolVersion::from_text(&fields[2])?,
            method: Method::from_text(&fields[3])?,
            payload: Payload::new(unescape(&fields[4])?),
        })
    }
}

/// Correlated response. Exactly one of `result` and `error` must be present;
/// [`Response::validate`] enforces that invariant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    id: RequestId,
    result: Option<String>,
    error: Option<ProtocolError>,
}

impl Response {
    /// unchecked constructor, used for validation tests and parsers. Call
    /// [`Self::validate`] before trusting the value.
    pub fn new(id: RequestId, result: Option<String>, error: Option<ProtocolError>) -> Self {
        Self { id, result, error }
    }

    /// Successful response.
    pub fn ok(id: RequestId, result: impl Into<String>) -> Self {
        Self::new(id, Some(result.into()), None)
    }

    /// Error response. The error is always a fixed taxonomy variant.
    pub fn err(id: RequestId, error: ProtocolError) -> Self {
        Self::new(id, None, Some(error))
    }

    /// Correlation identifier.
    pub fn id(&self) -> RequestId {
        self.id
    }

    /// Successful result text, if present.
    pub fn result(&self) -> Option<&str> {
        self.result.as_deref()
    }

    /// Protocol error, if present.
    pub fn error(&self) -> Option<ProtocolError> {
        self.error
    }

    /// Exactly one of result/error, and error codes are always known.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        match (&self.result, self.error) {
            (Some(_), None) | (None, Some(_)) => Ok(()),
            _ => Err(ProtocolError::InvalidPayload),
        }
    }

    /// Canonical compact text form: `RES|id|OK|result` or `RES|id|ERR|CODE`.
    pub fn to_text(&self) -> String {
        match (&self.result, self.error) {
            (Some(result), None) => {
                format!("{}|{}|{}|{}", RESPONSE_TAG, self.id, OK_TAG, escape(result))
            }
            (None, Some(error)) => {
                format!("{}|{}|{}|{}", RESPONSE_TAG, self.id, ERR_TAG, error)
            }
            _ => format!("{}|{}|INVALID|", RESPONSE_TAG, self.id),
        }
    }

    /// Parse a canonical response text form.
    pub fn from_text(text: &str) -> Result<Self, ProtocolError> {
        let fields = split_fields(text)?;
        if fields.len() != 4 || fields[0] != RESPONSE_TAG {
            return Err(ProtocolError::InvalidPayload);
        }
        let id = RequestId::from_text(&fields[1])?;
        match fields[2].as_str() {
            OK_TAG => Ok(Self::ok(id, unescape(&fields[3])?)),
            ERR_TAG => Ok(Self::err(id, ProtocolError::from_text(&fields[3])?)),
            _ => Err(ProtocolError::InvalidPayload),
        }
    }
}

/// Escape `\`, `|`, newline and carriage return so text forms are unambiguous.
fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}

/// Inverse of [`escape`]; rejects dangling or unknown escape sequences.
fn unescape(input: &str) -> Result<String, ProtocolError> {
    let mut chars = input.chars();
    let mut out = String::with_capacity(input.len());
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('|') => out.push('|'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            _ => return Err(ProtocolError::InvalidPayload),
        }
    }
    Ok(out)
}

/// Split on unescaped `|`, preserving escape sequences for [`unescape`].
fn split_fields(input: &str) -> Result<Vec<String>, ProtocolError> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for ch in input.chars() {
        if escaped {
            current.push('\\');
            current.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '|' => fields.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    if escaped {
        return Err(ProtocolError::InvalidPayload);
    }
    fields.push(current);
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HANDLE: &str = "pv-handle-sim-SECRET-4242";

    #[test]
    fn request_id_display_and_parse_roundtrip() {
        let id = RequestId::from_seed(0x2a);
        let text = id.to_text();
        assert_eq!(text, "r000000000000002a");
        assert_eq!(RequestId::from_text(&text).unwrap(), id);
        assert_eq!(text, id.to_string());
    }

    #[test]
    fn request_id_new_is_fresh() {
        assert_ne!(RequestId::new(), RequestId::new());
    }

    #[test]
    fn request_id_parse_rejects_garbage() {
        for bad in ["", "r", "rxyz", "0001", "r00000000000000001", "rgg"] {
            assert_eq!(
                RequestId::from_text(bad),
                Err(ProtocolError::InvalidPayload)
            );
        }
    }

    #[test]
    fn version_v1_negotiates() {
        assert_eq!(
            ProtocolVersion::V1.negotiate().unwrap(),
            ProtocolVersion::V1
        );
        assert_eq!(
            ProtocolVersion::from_number(1).negotiate().unwrap(),
            ProtocolVersion::V1
        );
    }

    #[test]
    fn unsupported_version_is_rejected() {
        for number in [0, 2, 99, u16::MAX] {
            assert_eq!(
                ProtocolVersion::from_number(number).negotiate(),
                Err(ProtocolError::UnsupportedVersion)
            );
        }
    }

    #[test]
    fn version_text_roundtrip_and_parse_failure() {
        assert_eq!(ProtocolVersion::V1.to_text(), "v1");
        assert_eq!(
            ProtocolVersion::from_text("v1").unwrap(),
            ProtocolVersion::V1
        );
        assert_eq!(
            ProtocolVersion::from_text("v2").unwrap().negotiate(),
            Err(ProtocolError::UnsupportedVersion)
        );
        assert_eq!(
            ProtocolVersion::from_text("vX"),
            Err(ProtocolError::InvalidPayload)
        );
    }

    #[test]
    fn request_validate_rejects_unsupported_version() {
        let request = Request::new(
            RequestId::from_seed(9),
            ProtocolVersion::from_number(2),
            Method::ListAccounts,
            Payload::EMPTY,
        );
        assert_eq!(request.validate(), Err(ProtocolError::UnsupportedVersion));
    }

    #[test]
    fn all_method_variants_have_stable_text_forms() {
        let expected = [
            (Method::ListAccounts, "list_accounts"),
            (Method::GetBalance, "get_balance"),
            (Method::Approve, "approve"),
            (Method::SignMessage, "sign_message"),
            (Method::Transfer, "transfer"),
        ];
        assert_eq!(Method::all().len(), expected.len());
        for (method, token) in expected {
            assert_eq!(method.as_str(), token);
            assert_eq!(Method::from_text(token).unwrap(), method);
            assert_eq!(method.to_string(), token);
        }
    }

    #[test]
    fn method_parse_rejects_unknown_name() {
        assert_eq!(
            Method::from_text("drain_accounts"),
            Err(ProtocolError::UnknownMethod)
        );
        assert_eq!(Method::from_text(""), Err(ProtocolError::UnknownMethod));
    }

    #[test]
    fn request_text_roundtrip_is_deterministic() {
        let request = Request::new(
            RequestId::from_seed(7),
            ProtocolVersion::V1,
            Method::GetBalance,
            Payload::new("account=local-1"),
        );
        let text = request.to_text();
        assert_eq!(text, "REQ|r0000000000000007|v1|get_balance|account=local-1");
        assert_eq!(text, request.to_text());
        assert_eq!(Request::from_text(&text).unwrap(), request);
    }

    #[test]
    fn payload_escaping_roundtrips_delimiters() {
        let payload = Payload::new(format!("a|b\\c\nd\r{HANDLE}"));
        let request = Request::new(
            RequestId::from_seed(8),
            ProtocolVersion::V1,
            Method::SignMessage,
            payload.clone(),
        );
        let text = request.to_text();
        assert!(!text.contains("a|b"));
        assert_eq!(Request::from_text(&text).unwrap().payload(), &payload);
        assert_eq!(
            Request::from_text("REQ|r1|v1|transfer|bad\\q"),
            Err(ProtocolError::InvalidPayload)
        );
    }

    #[test]
    fn response_ok_text_roundtrip() {
        let response = Response::ok(RequestId::from_seed(3), "balance=0");
        let text = response.to_text();
        assert_eq!(text, "RES|r0000000000000003|OK|balance=0");
        assert_eq!(Response::from_text(&text).unwrap(), response);
        assert_eq!(response.validate(), Ok(()));
    }

    #[test]
    fn response_error_text_roundtrip_and_full_taxonomy() {
        for error in ProtocolError::all() {
            let response = Response::err(RequestId::from_seed(4), error);
            let text = response.to_text();
            assert!(text.ends_with(error.as_str()));
            assert_eq!(Response::from_text(&text).unwrap(), response);
            assert_eq!(response.validate(), Ok(()));
        }
    }

    #[test]
    fn response_rejects_both_result_and_error() {
        let response = Response::new(
            RequestId::from_seed(5),
            Some("value".to_owned()),
            Some(ProtocolError::Denied),
        );
        assert_eq!(response.validate(), Err(ProtocolError::InvalidPayload));
        assert_eq!(
            Response::from_text("RES|r5|OK|a|ERR|DENIED"),
            Err(ProtocolError::InvalidPayload)
        );
    }

    #[test]
    fn response_rejects_neither_result_nor_error() {
        let response = Response::new(RequestId::from_seed(6), None, None);
        assert_eq!(response.validate(), Err(ProtocolError::InvalidPayload));
    }

    #[test]
    fn response_text_parse_rejects_invalid_shapes() {
        let bad = [
            "RES|r1|MAYBE|value",
            "RES|r1|ERR|UNKNOWN_CODE",
            "RES|not-an-id|OK|value",
            "REQ|r1|v1|approve|",
            "",
        ];
        for text in bad {
            assert_eq!(
                Response::from_text(text),
                Err(ProtocolError::InvalidPayload)
            );
        }
    }

    #[test]
    fn error_taxonomy_marks_only_transport_failures_transient() {
        assert!(ProtocolError::Transport.is_transient());
        assert!(ProtocolError::Timeout.is_transient());
        for error in [
            ProtocolError::UnsupportedVersion,
            ProtocolError::UnknownMethod,
            ProtocolError::Cancelled,
            ProtocolError::Denied,
            ProtocolError::InvalidPayload,
        ] {
            assert!(!error.is_transient());
        }
    }

    #[test]
    fn errors_never_contain_handles_or_private_payloads() {
        let payload = Payload::new(format!("handle={HANDLE}"));
        let request = Request::new(
            RequestId::from_seed(11),
            ProtocolVersion::V1,
            Method::Transfer,
            payload,
        );
        for error in ProtocolError::all() {
            assert!(!error.to_string().contains(HANDLE));
            assert!(!format!("{error:?}").contains(HANDLE));
            let response = Response::err(request.id(), error);
            assert!(!response.to_text().contains(HANDLE));
        }
        // The payload is carried verbatim by the request, but the protocol
        // never interprets it and no error derived from it can disclose it.
        assert!(request.to_text().contains(HANDLE));
    }
}
