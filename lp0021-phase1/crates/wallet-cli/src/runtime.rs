// SPDX-License-Identifier: MIT AND Apache-2.0
//! Adapter-based wallet runtime: proposals are inert until an exact, explicit
//! approval. The small approval state machine and deny-by-default consent gate
//! are local reimplementations of the phase-0 / consent-store patterns.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use wallet_chain::{
    transaction_hash, AccountId, AccountKind, Asset, HistoryEntry, Ledger, LedgerError,
    NativeAsset, ReceiveHandle, ReplayError, Transaction,
};

use crate::approval::{
    ApprovalAuthorizer, ApprovalState, ApprovalToken, Denied, EffectsSummary, ProposalBinding,
    RequestId, SessionId,
};
use crate::consent::{
    Capability, ConsentError, ConsentGate, GrantId, PermissionStore, Scope, StoreGate,
};

/// Failures from runtime operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalletError {
    /// Deny-by-default consent check failed.
    Consent(ConsentError),
    /// Explicit authorization was refused.
    Denied(Denied),
    /// Ledger validation failed.
    Ledger(LedgerError),
    /// Event-log reconstruction failed.
    Replay(ReplayError),
    /// No proposal has this id.
    UnknownRequest(RequestId),
    /// Approval attempted in the wrong state.
    InvalidState {
        request_id: RequestId,
        expected: &'static str,
        actual: &'static str,
    },
    /// The supplied binding does not match the stored proposal.
    WrongRequestFields,
    /// The approver is not the proposal's sender.
    WrongApprover {
        expected: AccountId,
        found: AccountId,
    },
    /// A mock signer failed after approval.
    Signer(String),
}

impl Display for WalletError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Consent(error) => write!(f, "{error}"),
            Self::Denied(error) => write!(f, "{error}"),
            Self::Ledger(error) => write!(f, "{error}"),
            Self::Replay(error) => write!(f, "{error}"),
            Self::UnknownRequest(id) => write!(f, "unknown proposal {id}"),
            Self::InvalidState {
                request_id,
                expected,
                actual,
            } => write!(f, "proposal {request_id} is {actual}, expected {expected}"),
            Self::WrongRequestFields => write!(f, "approval fields do not match the proposal"),
            Self::WrongApprover { expected, found } => {
                write!(f, "wrong approver {found}: expected {expected}")
            }
            Self::Signer(reason) => write!(f, "mock signer failed: {reason}"),
        }
    }
}

impl std::error::Error for WalletError {}

fn mock_fnv(bytes: &[u8], seed: u64) -> u64 {
    let mut hash = seed;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Deterministic mock signature. HONESTY: this is not cryptography, not a
/// keypair signature, and provides no authenticity or confidentiality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MockSignature([u8; 32]);

impl MockSignature {
    /// Derive a mock signature from a transaction digest and approval token.
    pub fn derive(tx: &Transaction, token: &ApprovalToken) -> Self {
        let mut input = Vec::with_capacity(40 + token.as_str().len());
        input.extend_from_slice(transaction_hash(tx).as_bytes());
        input.extend_from_slice(token.as_str().as_bytes());
        let mut bytes = [0_u8; 32];
        for (index, chunk) in bytes.chunks_mut(8).enumerate() {
            let seed = 0xcbf2_9ce4_8422_2325_u64.wrapping_add((index as u64).wrapping_mul(17));
            chunk.copy_from_slice(&mock_fnv(&input, seed).to_le_bytes());
        }
        Self(bytes)
    }

    /// Mock signature bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Display for MockSignature {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Signing boundary. In a real wallet this would wrap a keystore; here it is
/// explicitly a mock and is called only after an exact explicit approval.
pub trait SignerAdapter {
    /// Produce a mock signature for an approved transaction.
    fn sign(&self, tx: &Transaction, token: &ApprovalToken) -> Result<MockSignature, WalletError>;
}

/// Observed signer calls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SignerLog {
    pub sign_calls: usize,
}

/// Mock signer with an optional injected failure.
#[derive(Clone, Debug)]
pub struct MockSigner {
    log: Rc<RefCell<SignerLog>>,
    fail: bool,
}

impl Default for MockSigner {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSigner {
    /// Create a signer that always succeeds.
    pub fn new() -> Self {
        Self {
            log: Rc::new(RefCell::new(SignerLog::default())),
            fail: false,
        }
    }

    /// Create a signer that fails when called (for negative tests).
    pub fn failing() -> Self {
        Self {
            fail: true,
            ..Self::new()
        }
    }

    /// Snapshot of observed calls.
    pub fn log_snapshot(&self) -> SignerLog {
        *self.log.borrow()
    }
}

impl SignerAdapter for MockSigner {
    fn sign(&self, tx: &Transaction, token: &ApprovalToken) -> Result<MockSignature, WalletError> {
        self.log.borrow_mut().sign_calls += 1;
        if self.fail {
            return Err(WalletError::Signer("injected mock failure".to_owned()));
        }
        Ok(MockSignature::derive(tx, token))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ApprovalRecord {
    request_id: RequestId,
    tx: Transaction,
    effects: EffectsSummary,
    state: ApprovalState,
}

/// Result of building a proposal. No signature exists at this point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SendOutcome {
    pub request_id: RequestId,
    pub binding: ProposalBinding,
    pub effects: EffectsSummary,
}

/// The approved/sign/submit-capable wallet runtime.
pub struct WalletRuntime {
    ledger: Ledger,
    store: Rc<RefCell<PermissionStore>>,
    gate: Box<dyn ConsentGate>,
    authorizer: Box<dyn ApprovalAuthorizer>,
    signer: Box<dyn SignerAdapter>,
    dapp: crate::consent::DappId,
    session: SessionId,
    records: BTreeMap<RequestId, ApprovalRecord>,
    next_request: u64,
    next_receive: u64,
    sign_count: usize,
    submit_count: usize,
    last_signature: Option<MockSignature>,
}

impl WalletRuntime {
    /// Assemble a runtime from its adapters. The dApp identity is issued by the
    /// supplied store at construction time.
    pub fn new(
        ledger: Ledger,
        store: Rc<RefCell<PermissionStore>>,
        gate: Box<dyn ConsentGate>,
        authorizer: Box<dyn ApprovalAuthorizer>,
        signer: Box<dyn SignerAdapter>,
    ) -> Self {
        let dapp = store.borrow_mut().register_dapp("local-cli");
        Self {
            ledger,
            store,
            gate,
            authorizer,
            signer,
            dapp,
            session: SessionId::new("local-cli-session"),
            records: BTreeMap::new(),
            next_request: 1,
            next_receive: 1,
            sign_count: 0,
            submit_count: 0,
            last_signature: None,
        }
    }

    /// Shared store handle, useful for revocation and audit in tests.
    pub fn store(&self) -> Rc<RefCell<PermissionStore>> {
        self.store.clone()
    }

    /// dApp identity issued to this runtime.
    pub fn dapp(&self) -> crate::consent::DappId {
        self.dapp
    }

    /// Local session id.
    pub fn session(&self) -> &SessionId {
        &self.session
    }

    /// Issue one explicit capability grant.
    pub fn grant(&self, capability: Capability, scope: Scope) -> Result<GrantId, ConsentError> {
        self.store
            .borrow_mut()
            .grant(&self.dapp, capability, &scope)
    }

    /// Revoke one explicit grant.
    pub fn revoke(&self, id: GrantId) -> Result<(), ConsentError> {
        self.store.borrow_mut().revoke(id)
    }

    /// Open a simulated account in the underlying ledger.
    pub fn open_account(
        &mut self,
        kind: AccountKind,
        initial_native: u128,
    ) -> Result<AccountId, WalletError> {
        self.ledger
            .create_account(kind, initial_native)
            .map_err(WalletError::Ledger)
    }

    /// Native balance, gated by a `BalanceRead` grant for this account.
    pub fn balance(&self, account: AccountId) -> Result<u128, WalletError> {
        self.require(Capability::BalanceRead, &Scope::Account(account))?;
        self.ledger
            .native_balance(account)
            .map_err(WalletError::Ledger)
    }

    /// Aggregate native balance across accounts using [`NativeAsset`].
    pub fn native_total(&self, accounts: &[AccountId]) -> Result<u128, WalletError> {
        for account in accounts {
            self.require(Capability::BalanceRead, &Scope::Account(*account))?;
        }
        NativeAsset::new()
            .aggregate(&self.ledger, accounts)
            .map_err(WalletError::Ledger)
    }

    /// Per-account history, gated by a `BalanceRead` grant for that account.
    pub fn history(&self, account: AccountId) -> Result<Vec<HistoryEntry>, WalletError> {
        self.require(Capability::BalanceRead, &Scope::Account(account))?;
        self.ledger.history(account).map_err(WalletError::Ledger)
    }

    /// Generate an opaque receive handle. No account id, address or key is
    /// embedded in the handle.
    pub fn receive(&mut self, kind: AccountKind) -> Result<ReceiveHandle, WalletError> {
        self.require(Capability::Receive, &Scope::Unspecified)?;
        let handle = ReceiveHandle::new(self.next_receive, kind);
        self.next_receive += 1;
        Ok(handle)
    }

    /// Build a transaction and store an inert pending proposal. NOTHING is
    /// signed or applied to the ledger here.
    pub fn send(
        &mut self,
        from: AccountId,
        to: AccountId,
        amount: u128,
        asset: Asset,
    ) -> Result<SendOutcome, WalletError> {
        self.require(Capability::ProposeTransfer, &Scope::Account(from))?;
        let kind = self
            .ledger
            .account_kind(from)
            .map_err(WalletError::Ledger)?;
        if amount == 0 {
            return Err(WalletError::Ledger(LedgerError::ZeroAmount));
        }
        if from == to {
            return Err(WalletError::Ledger(LedgerError::SameAccount(from)));
        }
        let available = self
            .ledger
            .balance(from, asset.clone())
            .map_err(WalletError::Ledger)?;
        if available < amount {
            return Err(WalletError::Ledger(LedgerError::InsufficientBalance {
                account: from,
                asset,
                available,
                required: amount,
            }));
        }
        let tx = Transaction {
            from,
            to,
            amount,
            asset,
            nonce: self.proposed_nonce(from),
        };
        let effects = EffectsSummary::new(&tx, kind);
        let request_id = RequestId::new(self.next_request);
        self.next_request += 1;
        self.records.insert(
            request_id,
            ApprovalRecord {
                request_id,
                tx: tx.clone(),
                effects: effects.clone(),
                state: ApprovalState::AwaitingApproval,
            },
        );
        Ok(SendOutcome {
            request_id,
            binding: ProposalBinding::from_tx(&tx),
            effects,
        })
    }

    /// Effects preview for a proposal.
    pub fn effects_preview(&self, id: RequestId) -> Result<&EffectsSummary, WalletError> {
        Ok(&self.record(id)?.effects)
    }

    /// Current approval state of a proposal.
    pub fn state(&self, id: RequestId) -> Result<&ApprovalState, WalletError> {
        Ok(&self.record(id)?.state)
    }

    /// Number of proposals still awaiting a decision.
    pub fn pending_count(&self) -> usize {
        self.records
            .values()
            .filter(|record| record.state == ApprovalState::AwaitingApproval)
            .count()
    }

    /// Number of mock signer invocations observed so far.
    pub const fn sign_count(&self) -> usize {
        self.sign_count
    }

    /// Number of ledger submissions observed so far.
    pub const fn submission_count(&self) -> usize {
        self.submit_count
    }

    /// Event log length in the underlying ledger.
    pub fn ledger_event_count(&self) -> usize {
        self.ledger.events().len()
    }

    /// Last mock signature produced, if any. Not a real cryptographic value.
    pub const fn last_signature(&self) -> Option<MockSignature> {
        self.last_signature
    }

    /// Reject a proposal. A rejected proposal can never be approved later.
    pub fn reject(&mut self, id: RequestId, reason: impl Into<String>) -> Result<(), WalletError> {
        let record = self
            .records
            .get_mut(&id)
            .ok_or(WalletError::UnknownRequest(id))?;
        if record.state != ApprovalState::AwaitingApproval {
            return Err(WalletError::InvalidState {
                request_id: id,
                expected: "AwaitingApproval",
                actual: record.state.name(),
            });
        }
        record.state = ApprovalState::Rejected {
            reason: reason.into(),
        };
        Ok(())
    }

    /// The only path that signs and submits. Every guard must pass for the
    /// *exact* proposal: state, binding, approver, consent and explicit grant.
    pub fn approve(
        &mut self,
        id: RequestId,
        binding: &ProposalBinding,
        approver: AccountId,
    ) -> Result<wallet_chain::TxReceipt, WalletError> {
        let record = self
            .records
            .get(&id)
            .ok_or(WalletError::UnknownRequest(id))?;
        if record.state != ApprovalState::AwaitingApproval {
            return Err(WalletError::InvalidState {
                request_id: id,
                expected: "AwaitingApproval",
                actual: record.state.name(),
            });
        }
        let expected_binding = ProposalBinding::from_tx(&record.tx);
        if *binding != expected_binding {
            return Err(WalletError::WrongRequestFields);
        }
        if approver != expected_binding.from {
            return Err(WalletError::WrongApprover {
                expected: expected_binding.from,
                found: approver,
            });
        }
        let tx = record.tx.clone();
        let effects = record.effects.clone();

        self.require(Capability::SignTransaction, &Scope::Account(approver))?;
        let token = self
            .authorizer
            .approve(&self.session, &approver, &effects)
            .map_err(WalletError::Denied)?;
        if let Some(record) = self.records.get_mut(&id) {
            record.state = ApprovalState::Approved {
                token: token.clone(),
            };
        }

        let signature = match self.signer.sign(&tx, &token) {
            Ok(signature) => signature,
            Err(error) => {
                self.fail(id, error.to_string());
                return Err(error);
            }
        };
        self.sign_count += 1;
        self.last_signature = Some(signature);

        let receipt = match self.ledger.apply(tx) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.fail(id, error.to_string());
                return Err(WalletError::Ledger(error));
            }
        };
        self.submit_count += 1;
        if let Some(record) = self.records.get_mut(&id) {
            record.state = ApprovalState::Submitted {
                token,
                hash: receipt.hash,
            };
        }
        Ok(receipt)
    }

    /// Simulate a process restart: rebuild the ledger from its event log and
    /// discard volatile pending proposals. Returns the replayed event count.
    pub fn restart(&mut self) -> Result<usize, WalletError> {
        let events = self.ledger.events().to_vec();
        self.ledger = Ledger::replay(&events).map_err(WalletError::Replay)?;
        self.records.clear();
        self.next_request = 1;
        self.next_receive = 1;
        Ok(events.len())
    }

    fn record(&self, id: RequestId) -> Result<&ApprovalRecord, WalletError> {
        self.records.get(&id).ok_or(WalletError::UnknownRequest(id))
    }

    fn fail(&mut self, id: RequestId, reason: String) {
        if let Some(record) = self.records.get_mut(&id) {
            record.state = ApprovalState::Failed { reason };
        }
    }

    fn require(&self, capability: Capability, scope: &Scope) -> Result<(), WalletError> {
        self.gate
            .check(&self.dapp, capability, scope)
            .map_err(WalletError::Consent)
    }

    fn proposed_nonce(&self, from: AccountId) -> u64 {
        let pending = self
            .records
            .values()
            .filter(|record| {
                record.tx.from == from
                    && matches!(
                        record.state,
                        ApprovalState::AwaitingApproval | ApprovalState::Approved { .. }
                    )
            })
            .count() as u64;
        self.ledger
            .next_nonce(from)
            .unwrap_or(0)
            .saturating_add(pending)
    }
}

/// Deterministic local demo runtime with three accounts:
/// `acct-1` public (1000 native), `acct-2` private (1000 native), `acct-3`
/// public (0 native). Capabilities are explicitly granted to the local CLI
/// dApp at startup; each transfer still needs its own `approve` command.
pub fn demo_runtime() -> WalletRuntime {
    let mut ledger = Ledger::new();
    let accounts = [
        ledger
            .create_account(AccountKind::Public, 1000)
            .expect("local fixture"),
        ledger
            .create_account(AccountKind::Private, 1000)
            .expect("local fixture"),
        ledger
            .create_account(AccountKind::Public, 0)
            .expect("local fixture"),
    ];
    let store = Rc::new(RefCell::new(PermissionStore::new()));
    let gate = StoreGate::new(store.clone());
    let runtime = WalletRuntime::new(
        ledger,
        store,
        Box::new(gate),
        Box::new(crate::approval::ExplicitApprover::new()),
        Box::new(MockSigner::new()),
    );
    for account in accounts {
        for capability in [
            Capability::ProposeTransfer,
            Capability::SignTransaction,
            Capability::BalanceRead,
        ] {
            runtime
                .grant(capability, Scope::Account(account))
                .expect("local fixture grant");
        }
    }
    runtime
        .grant(Capability::Receive, Scope::Unspecified)
        .expect("local fixture grant");
    runtime
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::{ApprovalState, ExplicitApprover};
    use wallet_chain::AccountKind;

    struct Fixture {
        runtime: WalletRuntime,
        signer: MockSigner,
    }

    fn fixture() -> Fixture {
        let mut ledger = Ledger::new();
        let alice = ledger.create_account(AccountKind::Public, 100).unwrap();
        let bob = ledger.create_account(AccountKind::Private, 100).unwrap();
        let store = Rc::new(RefCell::new(PermissionStore::new()));
        let gate = StoreGate::new(store.clone());
        let signer = MockSigner::new();
        let runtime = WalletRuntime::new(
            ledger,
            store.clone(),
            Box::new(gate),
            Box::new(ExplicitApprover::new()),
            Box::new(signer.clone()),
        );
        let dapp = runtime.dapp();
        for account in [alice, bob] {
            for capability in [
                Capability::ProposeTransfer,
                Capability::SignTransaction,
                Capability::BalanceRead,
            ] {
                store
                    .borrow_mut()
                    .grant(&dapp, capability, &Scope::Account(account))
                    .unwrap();
            }
        }
        store
            .borrow_mut()
            .grant(&dapp, Capability::Receive, &Scope::Unspecified)
            .unwrap();
        Fixture { runtime, signer }
    }

    fn send(f: &mut Fixture) -> (RequestId, ProposalBinding) {
        let outcome = f
            .runtime
            .send(AccountId::new(1), AccountId::new(2), 10, Asset::Native)
            .unwrap();
        (outcome.request_id, outcome.binding)
    }

    #[test]
    fn send_only_creates_a_pending_proposal() {
        let mut f = fixture();
        let (id, _) = send(&mut f);
        assert_eq!(f.runtime.state(id), Ok(&ApprovalState::AwaitingApproval));
        assert_eq!(f.runtime.pending_count(), 1);
        assert_eq!(f.runtime.sign_count(), 0);
        assert_eq!(f.runtime.submission_count(), 0);
        assert_eq!(f.signer.log_snapshot().sign_calls, 0);
        assert_eq!(f.runtime.balance(AccountId::new(1)), Ok(100));
        assert_eq!(f.runtime.balance(AccountId::new(2)), Ok(100));
    }

    #[test]
    fn no_approval_means_no_signature_and_no_submission() {
        let mut f = fixture();
        let _ = send(&mut f);
        assert_eq!(f.runtime.sign_count(), 0);
        assert_eq!(f.runtime.submission_count(), 0);
        assert_eq!(f.runtime.ledger_event_count(), 4);
    }

    #[test]
    fn exact_approval_signs_and_submits_once() {
        let mut f = fixture();
        let (id, binding) = send(&mut f);
        let receipt = f.runtime.approve(id, &binding, AccountId::new(1)).unwrap();
        assert_eq!(
            f.runtime.state(id),
            Ok(&ApprovalState::Submitted {
                token: ApprovalToken::new("approval-token-1"),
                hash: receipt.hash,
            })
        );
        assert_eq!(f.runtime.sign_count(), 1);
        assert_eq!(f.runtime.submission_count(), 1);
        assert_eq!(f.signer.log_snapshot().sign_calls, 1);
        assert_eq!(f.runtime.balance(AccountId::new(1)), Ok(90));
        assert_eq!(f.runtime.balance(AccountId::new(2)), Ok(110));
    }

    #[test]
    fn approval_replay_fails() {
        let mut f = fixture();
        let (id, binding) = send(&mut f);
        f.runtime.approve(id, &binding, AccountId::new(1)).unwrap();
        assert!(matches!(
            f.runtime.approve(id, &binding, AccountId::new(1)),
            Err(WalletError::InvalidState { .. })
        ));
        assert_eq!(f.runtime.sign_count(), 1);
        assert_eq!(f.runtime.submission_count(), 1);
    }

    #[test]
    fn wrong_approver_fails_before_signing() {
        let mut f = fixture();
        let (id, binding) = send(&mut f);
        assert_eq!(
            f.runtime.approve(id, &binding, AccountId::new(2)),
            Err(WalletError::WrongApprover {
                expected: AccountId::new(1),
                found: AccountId::new(2),
            })
        );
        assert_eq!(f.runtime.sign_count(), 0);
        assert_eq!(f.runtime.state(id), Ok(&ApprovalState::AwaitingApproval));
    }

    #[test]
    fn wrong_amount_nonce_or_destination_binding_fails() {
        let mut f = fixture();
        let (id, binding) = send(&mut f);
        for changed in [
            ProposalBinding {
                amount: 11,
                ..binding.clone()
            },
            ProposalBinding {
                nonce: 7,
                ..binding.clone()
            },
            ProposalBinding {
                to: AccountId::new(9),
                ..binding.clone()
            },
            ProposalBinding {
                asset: Asset::MockToken(1),
                ..binding.clone()
            },
        ] {
            assert_eq!(
                f.runtime.approve(id, &changed, AccountId::new(1)),
                Err(WalletError::WrongRequestFields)
            );
        }
        assert_eq!(f.runtime.sign_count(), 0);
    }

    #[test]
    fn reject_then_approve_fails() {
        let mut f = fixture();
        let (id, binding) = send(&mut f);
        f.runtime.reject(id, "changed my mind").unwrap();
        assert!(matches!(
            f.runtime.state(id),
            Ok(ApprovalState::Rejected { .. })
        ));
        assert!(matches!(
            f.runtime.approve(id, &binding, AccountId::new(1)),
            Err(WalletError::InvalidState { .. })
        ));
        assert_eq!(f.runtime.sign_count(), 0);
        assert_eq!(f.runtime.submission_count(), 0);
    }

    #[test]
    fn insufficient_balance_is_rejected_at_proposal_time() {
        let mut f = fixture();
        assert!(matches!(
            f.runtime
                .send(AccountId::new(1), AccountId::new(2), 101, Asset::Native),
            Err(WalletError::Ledger(LedgerError::InsufficientBalance { .. }))
        ));
        assert_eq!(f.runtime.pending_count(), 0);
        assert_eq!(f.runtime.sign_count(), 0);
    }

    #[test]
    fn deny_by_default_blocks_proposal_and_approval() {
        let mut ledger = Ledger::new();
        let alice = ledger.create_account(AccountKind::Public, 10).unwrap();
        let bob = ledger.create_account(AccountKind::Public, 0).unwrap();
        let store = Rc::new(RefCell::new(PermissionStore::new()));
        let gate = StoreGate::new(store.clone());
        let mut runtime = WalletRuntime::new(
            ledger,
            store,
            Box::new(gate),
            Box::new(ExplicitApprover::new()),
            Box::new(MockSigner::new()),
        );
        assert!(matches!(
            runtime.send(alice, bob, 1, Asset::Native),
            Err(WalletError::Consent(ConsentError::Denied))
        ));
        assert!(matches!(
            runtime.balance(alice),
            Err(WalletError::Consent(ConsentError::Denied))
        ));
    }

    #[test]
    fn revoking_sign_capability_blocks_approval() {
        let mut f = fixture();
        let dapp = f.runtime.dapp();
        let grants = f.runtime.store().borrow().list_for(&dapp).len();
        assert_eq!(grants, 7);
        // The SignTransaction grant for acct-1 is the second grant issued for
        // that account; revoke by scanning through the store's own audit text.
        let lines = f.runtime.store().borrow().list_for(&dapp);
        let sign_line = lines
            .iter()
            .find(|line| line.contains("transaction signature") && line.contains("acct-1"))
            .unwrap();
        let grant_text = sign_line.split(':').next().unwrap();
        let id = grant_text.parse::<crate::consent::GrantId>().unwrap();
        f.runtime.revoke(id).unwrap();
        let (request, binding) = send(&mut f);
        assert!(matches!(
            f.runtime.approve(request, &binding, AccountId::new(1)),
            Err(WalletError::Consent(_))
        ));
        assert_eq!(f.runtime.sign_count(), 0);
    }

    #[test]
    fn wrong_scope_is_reported_instead_of_plain_denial() {
        let f = fixture();
        let dapp = f.runtime.dapp();
        let wrong = AccountId::new(999);
        assert_eq!(
            f.runtime.store().borrow().check(
                &dapp,
                Capability::ProposeTransfer,
                &Scope::Account(wrong)
            ),
            Err(ConsentError::ScopeMismatch)
        );
    }

    #[test]
    fn public_and_private_sends_have_same_balance_invariants() {
        let mut f = fixture();
        let public = f
            .runtime
            .send(AccountId::new(1), AccountId::new(2), 7, Asset::Native)
            .unwrap();
        f.runtime
            .approve(public.request_id, &public.binding, AccountId::new(1))
            .unwrap();
        let private = f
            .runtime
            .send(AccountId::new(2), AccountId::new(1), 7, Asset::Native)
            .unwrap();
        f.runtime
            .approve(private.request_id, &private.binding, AccountId::new(2))
            .unwrap();
        assert_eq!(f.runtime.balance(AccountId::new(1)), Ok(100));
        assert_eq!(f.runtime.balance(AccountId::new(2)), Ok(100));
        let events = f.runtime.ledger_event_count();
        assert_eq!(events, 6);
        assert_eq!(
            f.runtime
                .native_total(&[AccountId::new(1), AccountId::new(2)]),
            Ok(200)
        );
    }

    #[test]
    fn restart_replays_events_and_drops_pending_proposals() {
        let mut f = fixture();
        let (id, binding) = send(&mut f);
        f.runtime.approve(id, &binding, AccountId::new(1)).unwrap();
        let _pending = send(&mut f);
        let count = f.runtime.restart().unwrap();
        assert_eq!(count, 5);
        assert_eq!(f.runtime.pending_count(), 0);
        assert_eq!(f.runtime.balance(AccountId::new(1)), Ok(90));
        assert_eq!(f.runtime.balance(AccountId::new(2)), Ok(110));
        assert!(matches!(
            f.runtime.state(id),
            Err(WalletError::UnknownRequest(_))
        ));
    }

    #[test]
    fn receive_generates_opaque_handles() {
        let mut f = fixture();
        let public = f.runtime.receive(AccountKind::Public).unwrap();
        let private = f.runtime.receive(AccountKind::Private).unwrap();
        assert_ne!(public, private);
        assert!(!public.to_string().contains("acct-"));
    }

    #[test]
    fn signer_failure_marks_proposal_failed_without_submission() {
        let mut ledger = Ledger::new();
        let alice = ledger.create_account(AccountKind::Public, 10).unwrap();
        let bob = ledger.create_account(AccountKind::Public, 0).unwrap();
        let store = Rc::new(RefCell::new(PermissionStore::new()));
        let gate = StoreGate::new(store.clone());
        let mut runtime = WalletRuntime::new(
            ledger,
            store.clone(),
            Box::new(gate),
            Box::new(ExplicitApprover::new()),
            Box::new(MockSigner::failing()),
        );
        let dapp = runtime.dapp();
        for capability in [
            Capability::ProposeTransfer,
            Capability::SignTransaction,
            Capability::BalanceRead,
        ] {
            store
                .borrow_mut()
                .grant(&dapp, capability, &Scope::Account(alice))
                .unwrap();
        }
        let outcome = runtime.send(alice, bob, 1, Asset::Native).unwrap();
        assert!(matches!(
            runtime.approve(outcome.request_id, &outcome.binding, alice),
            Err(WalletError::Signer(_))
        ));
        assert!(matches!(
            runtime.state(outcome.request_id),
            Ok(ApprovalState::Failed { .. })
        ));
        assert_eq!(runtime.submission_count(), 0);
        assert_eq!(runtime.balance(alice), Ok(10));
        assert_eq!(
            runtime.balance(bob),
            Err(WalletError::Consent(ConsentError::ScopeMismatch))
        );
    }
}
