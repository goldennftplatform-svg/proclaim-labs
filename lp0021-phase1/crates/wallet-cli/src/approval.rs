// SPDX-License-Identifier: MIT AND Apache-2.0
//! Local reimplementation of the phase-0 approval state machine surface.
//!
//! A proposal is inert until the explicit `approve` decision. The state
//! machine rejects approval replay, wrong-account approval and approval after
//! rejection; only `AwaitingApproval` can transition to `Approved`.

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;
use std::str::FromStr;

use wallet_chain::{transaction_hash, AccountId, AccountKind, Asset, Transaction, TxHash};

use crate::consent::ConsentError;

/// Opaque per-request id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(u64);

impl RequestId {
    /// Construct from a serial.
    pub const fn new(serial: u64) -> Self {
        Self(serial)
    }

    /// Serial form.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Display for RequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "req-{:04}", self.0)
    }
}

/// Request id parse failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseRequestIdError;

impl Display for ParseRequestIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "expected a request id of the form req-N")
    }
}

impl std::error::Error for ParseRequestIdError {}

impl FromStr for RequestId {
    type Err = ParseRequestIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let digits = value.strip_prefix("req-").ok_or(ParseRequestIdError)?;
        let serial = digits.parse::<u64>().map_err(|_| ParseRequestIdError)?;
        Ok(Self(serial))
    }
}

/// Opaque session id for the local CLI session.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Construct a session id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Text form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Token produced only by a successful explicit approval decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalToken(String);

impl ApprovalToken {
    /// Construct a mock token. Real tokens must come from an authorizer.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Text form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ApprovalToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Human-readable description of what would happen if a proposal were
/// approved. This is the local analogue of phase-0's `EffectsSummary`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectsSummary {
    pub from: AccountId,
    pub to: AccountId,
    pub amount: u128,
    pub asset: Asset,
    pub nonce: u64,
    pub account_kind: AccountKind,
    pub tx_hash: TxHash,
    pub warnings: Vec<String>,
}

impl EffectsSummary {
    /// Build a preview from a transaction without changing ledger state.
    pub fn new(tx: &Transaction, account_kind: AccountKind) -> Self {
        let mut warnings = vec![
            "LOCAL SIMULATION ONLY: no LEZ zone, sequencer or testnet 0.4 is involved".to_owned(),
        ];
        if account_kind == AccountKind::Private {
            warnings.push(
                "simulated private account: opaque handle only, NOT real cryptography or a shielded pool"
                    .to_owned(),
            );
        }
        Self {
            from: tx.from,
            to: tx.to,
            amount: tx.amount,
            asset: tx.asset.clone(),
            nonce: tx.nonce,
            account_kind,
            tx_hash: transaction_hash(tx),
            warnings,
        }
    }

    /// Lines suitable for an approval prompt or CLI output.
    pub fn human_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("from: {}", self.from),
            format!("to: {}", self.to),
            format!("amount: {} {}", self.amount, self.asset),
            format!("nonce: {}", self.nonce),
            format!("account kind: {}", self.account_kind),
            format!("simulated digest: {}", self.tx_hash),
        ];
        lines.extend(self.warnings.iter().cloned());
        lines
    }
}

/// The exact fields an `approve` call must bind to. Changing any field makes
/// the approval fail before a signer is invoked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalBinding {
    pub from: AccountId,
    pub to: AccountId,
    pub amount: u128,
    pub asset: Asset,
    pub nonce: u64,
}

impl ProposalBinding {
    /// Extract an exact binding from a transaction.
    pub fn from_tx(tx: &Transaction) -> Self {
        Self {
            from: tx.from,
            to: tx.to,
            amount: tx.amount,
            asset: tx.asset.clone(),
            nonce: tx.nonce,
        }
    }
}

/// Approval state machine states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApprovalState {
    /// Waiting for an explicit per-request decision. Nothing is signed.
    AwaitingApproval,
    /// Explicit decision granted a token; signing/submission may proceed.
    Approved { token: ApprovalToken },
    /// The user rejected this exact request. Terminal.
    Rejected { reason: String },
    /// Signed and submitted to the in-memory ledger. Terminal.
    Submitted { token: ApprovalToken, hash: TxHash },
    /// A signer or ledger error occurred after approval. Terminal.
    Failed { reason: String },
}

impl ApprovalState {
    /// Stable state name for errors and output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::AwaitingApproval => "AwaitingApproval",
            Self::Approved { .. } => "Approved",
            Self::Rejected { .. } => "Rejected",
            Self::Submitted { .. } => "Submitted",
            Self::Failed { .. } => "Failed",
        }
    }
}

/// Explicit-authorization failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Denied {
    /// Consent store refused the capability/scope.
    Consent(ConsentError),
    /// The approver account is not the proposal's sender.
    WrongAccount,
    /// The supplied binding does not match the stored proposal.
    WrongRequestFields,
    /// The session does not match an authorized session.
    NoSession,
    /// The proposal is not in `AwaitingApproval`.
    InvalidState(String),
    /// The user (or an adapter acting as the user) declined.
    UserRejected,
}

impl Display for Denied {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Consent(error) => write!(f, "{error}"),
            Self::WrongAccount => write!(f, "approver is not the proposal sender"),
            Self::WrongRequestFields => write!(f, "approval fields do not match the proposal"),
            Self::NoSession => write!(f, "approval session is not authorized"),
            Self::InvalidState(state) => write!(f, "invalid approval state: {state}"),
            Self::UserRejected => write!(f, "explicit user authorization was not granted"),
        }
    }
}

impl std::error::Error for Denied {}

/// Adapter boundary for the explicit decision. Called only after the runtime
/// has checked state, exact binding, approver account and consent.
pub trait ApprovalAuthorizer {
    /// Decide whether to grant an approval token.
    fn approve(
        &self,
        session: &SessionId,
        account: &AccountId,
        effects: &EffectsSummary,
    ) -> Result<ApprovalToken, Denied>;
}

/// Number of authorizer calls and grants observed by tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AuthorizerLog {
    pub calls: usize,
    pub grants: usize,
}

/// Local explicit approver. It requires the local session and the proposal's
/// sender account; the runtime separately checks the exact field binding.
#[derive(Clone, Debug)]
pub struct ExplicitApprover {
    log: Rc<RefCell<AuthorizerLog>>,
}

impl Default for ExplicitApprover {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplicitApprover {
    /// Create an approver with an empty call log.
    pub fn new() -> Self {
        Self {
            log: Rc::new(RefCell::new(AuthorizerLog::default())),
        }
    }

    /// Snapshot of observed calls.
    pub fn log_snapshot(&self) -> AuthorizerLog {
        *self.log.borrow()
    }
}

impl ApprovalAuthorizer for ExplicitApprover {
    fn approve(
        &self,
        session: &SessionId,
        account: &AccountId,
        effects: &EffectsSummary,
    ) -> Result<ApprovalToken, Denied> {
        let mut log = self.log.borrow_mut();
        log.calls += 1;
        if session.as_str().is_empty() {
            return Err(Denied::NoSession);
        }
        if *account != effects.from {
            return Err(Denied::WrongAccount);
        }
        log.grants += 1;
        Ok(ApprovalToken::new(format!("approval-token-{}", log.calls)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx() -> Transaction {
        Transaction {
            from: AccountId::new(1),
            to: AccountId::new(2),
            amount: 5,
            asset: Asset::Native,
            nonce: 0,
        }
    }

    #[test]
    fn request_ids_round_trip_and_are_padded() {
        let id = RequestId::new(7);
        assert_eq!(id.to_string(), "req-0007");
        assert_eq!("req-0007".parse::<RequestId>(), Ok(id));
        assert!("nope".parse::<RequestId>().is_err());
    }

    #[test]
    fn effects_include_private_account_honesty_warning() {
        let effects = EffectsSummary::new(&tx(), AccountKind::Private);
        let lines = effects.human_lines().join("\n");
        assert!(lines.contains("NOT real cryptography"));
        assert!(lines.contains("LOCAL SIMULATION ONLY"));
        assert!(lines.contains("simulated digest"));
    }

    #[test]
    fn state_names_are_stable() {
        assert_eq!(ApprovalState::AwaitingApproval.name(), "AwaitingApproval");
        assert_eq!(
            ApprovalState::Failed {
                reason: "x".to_owned()
            }
            .name(),
            "Failed"
        );
    }

    #[test]
    fn explicit_approver_rejects_wrong_account_without_granting() {
        let approver = ExplicitApprover::new();
        let effects = EffectsSummary::new(&tx(), AccountKind::Public);
        let result = approver.approve(&SessionId::new("local"), &AccountId::new(99), &effects);
        assert_eq!(result, Err(Denied::WrongAccount));
        assert_eq!(
            approver.log_snapshot(),
            AuthorizerLog {
                calls: 1,
                grants: 0
            }
        );
    }
}
