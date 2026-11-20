//! LP-0021 Phase 0 approval state machine.
//!
//! Protocol invariant (plan decision 3 / reliability R2 / proposed Tier 2):
//! no transaction is signed or submitted unless the user has called
//! [`WalletCore::authorize`] for that specific request after the core has
//! produced an [`EffectsSummary`]. Registry and IDL lookups are optional
//! adapters; failures degrade to a raw-calldata preview and never imply
//! approval. No chain, sequencer or testnet behaviour is emulated here.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use wallet_types::{
    hexutil, AccountId, ApprovalState, ApprovalToken, Capability, Denied, EffectsSummary, IdlError,
    ProgramId, ProgramMeta, RegistryError, RequestId, SessionId, SignedTransaction,
    SimulationResult, SubmissionReceipt, TransactionRequest, VerificationStatus, WalletError,
    ZoneId,
};

/// Optional LP-0023 registry adapter.
pub trait ProgramRegistry {
    fn resolve(&self, program_id: &ProgramId) -> Result<ProgramMeta, RegistryError>;
    fn verification_status(&self, program_id: &ProgramId) -> VerificationStatus;
}

/// Optional IDL decode + simulate adapter (PR #188 `lez-idl` boundary).
pub trait IdlProvider {
    fn decode_and_simulate(
        &self,
        request: &TransactionRequest,
        meta: Option<&ProgramMeta>,
    ) -> Result<SimulationResult, IdlError>;
}

/// Policy / user-approval check. Called ONLY from `WalletCore::authorize`.
pub trait CoreAuthorizer {
    fn approve(
        &self,
        session: &SessionId,
        account: &AccountId,
        effects: &EffectsSummary,
    ) -> Result<ApprovalToken, Denied>;
}

/// Signing + submission boundary. In production this would wrap the keystore
/// and a LEZ node client; here it is a mock that records calls.
pub trait Signer {
    fn sign(
        &self,
        request_id: &RequestId,
        request: &TransactionRequest,
        effects: &EffectsSummary,
        token: &ApprovalToken,
    ) -> Result<SignedTransaction, WalletError>;
    fn submit(&self, tx: &SignedTransaction) -> Result<SubmissionReceipt, WalletError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApprovalRecord {
    request: TransactionRequest,
    state: ApprovalState,
}

/// The core wallet. All approval decisions are made here, never in a UI layer.
pub struct WalletCore {
    registry: Box<dyn ProgramRegistry>,
    idl: Box<dyn IdlProvider>,
    authorizer: Box<dyn CoreAuthorizer>,
    signer: Box<dyn Signer>,
    records: HashMap<RequestId, ApprovalRecord>,
    next_request: u64,
}

impl WalletCore {
    pub fn new(
        registry: Box<dyn ProgramRegistry>,
        idl: Box<dyn IdlProvider>,
        authorizer: Box<dyn CoreAuthorizer>,
        signer: Box<dyn Signer>,
    ) -> Self {
        Self {
            registry,
            idl,
            authorizer,
            signer,
            records: HashMap::new(),
            next_request: 0,
        }
    }

    /// `wallet_proposeTransaction`: resolve registry metadata, decode/simulate
    /// via the IDL, build the effects preview and wait for explicit approval.
    pub fn propose_transaction(
        &mut self,
        request: TransactionRequest,
    ) -> Result<RequestId, WalletError> {
        self.next_request += 1;
        let request_id = RequestId::new(format!("req-{}", self.next_request));

        let mut warnings: Vec<String> = Vec::new();
        let (meta, verification_status) = match self.registry.resolve(&request.program_id) {
            Ok(meta) => {
                let status = self.registry.verification_status(&request.program_id);
                match &status {
                    VerificationStatus::Mismatch => warnings.push(
                        "program source image does not match the deployed program".to_owned(),
                    ),
                    VerificationStatus::DeployerClaimUnverified => warnings.push(
                        "deployer source claim has not been independently verified".to_owned(),
                    ),
                    _ => {}
                }
                (Some(meta), status)
            }
            Err(err) => {
                warnings.push(format!(
                    "registry lookup failed ({}): showing unverified / unknown program",
                    err
                ));
                (None, VerificationStatus::Unregistered)
            }
        };

        let simulation = match self.idl.decode_and_simulate(&request, meta.as_ref()) {
            Ok(sim) => {
                warnings.extend(sim.notes.iter().cloned());
                sim
            }
            Err(err) => {
                warnings.push(format!("cannot render effects — IDL unavailable: {}", err));
                SimulationResult::degraded()
            }
        };

        let effects = build_effects(
            &request,
            meta.as_ref(),
            verification_status,
            simulation,
            warnings,
        );
        self.records.insert(
            request_id.clone(),
            ApprovalRecord {
                request,
                state: ApprovalState::AwaitingApproval { effects },
            },
        );
        Ok(request_id)
    }

    /// Human-readable preview for the approval dialog (`wallet_getEffectsPreview`).
    pub fn effects_preview(&self, request_id: &RequestId) -> Result<&EffectsSummary, WalletError> {
        self.record(request_id)?
            .state
            .effects()
            .ok_or_else(|| WalletError::InvalidState {
                request_id: request_id.clone(),
                expected: "an effects-bearing state".to_owned(),
                actual: self.record(request_id).unwrap().state.name().to_owned(),
            })
    }

    pub fn state(&self, request_id: &RequestId) -> Result<&ApprovalState, WalletError> {
        Ok(&self.record(request_id)?.state)
    }

    /// Reject a proposed transaction. A rejected request can never be signed.
    pub fn reject(
        &mut self,
        request_id: &RequestId,
        reason: Option<String>,
    ) -> Result<(), WalletError> {
        let record = self
            .records
            .get_mut(request_id)
            .ok_or_else(|| WalletError::UnknownRequest(request_id.clone()))?;
        match &record.state {
            ApprovalState::AwaitingApproval { .. } => {
                record.state = ApprovalState::Rejected { reason };
                Ok(())
            }
            other => Err(WalletError::InvalidState {
                request_id: request_id.clone(),
                expected: "AwaitingApproval".to_owned(),
                actual: other.name().to_owned(),
            }),
        }
    }

    /// The ONLY path that grants approval. Requires `Capability::ProposeTransaction`
    /// for the fee payer and an explicit decision from the `CoreAuthorizer`.
    pub fn authorize(
        &mut self,
        request_id: &RequestId,
        session: &SessionId,
        account: &AccountId,
        capability: Capability,
    ) -> Result<ApprovalToken, Denied> {
        if capability != Capability::ProposeTransaction {
            return Err(Denied::CapabilityNotGranted(capability));
        }
        let effects = match self.records.get(request_id).map(|r| &r.state) {
            None => return Err(Denied::InvalidState("unknown request".to_owned())),
            Some(ApprovalState::AwaitingApproval { effects }) => effects.clone(),
            Some(other) => {
                return Err(Denied::InvalidState(format!(
                    "expected AwaitingApproval, got {}",
                    other.name()
                )))
            }
        };
        if *account != self.records[request_id].request.fee_payer {
            return Err(Denied::AccountMismatch);
        }
        if *session != self.records[request_id].request.session {
            return Err(Denied::NoSession);
        }

        let token = self.authorizer.approve(session, account, &effects)?;
        if let Some(record) = self.records.get_mut(request_id) {
            record.state = ApprovalState::Approved {
                effects,
                token: token.clone(),
            };
        }
        Ok(token)
    }

    /// Sign and submit. Refuses unless the state is exactly `Approved`, so a
    /// direct call after `propose_transaction` can never reach the signer.
    pub fn sign_and_submit(
        &mut self,
        request_id: &RequestId,
    ) -> Result<SubmissionReceipt, WalletError> {
        let (effects, token) = match self.records.get(request_id).map(|r| &r.state) {
            None => return Err(WalletError::UnknownRequest(request_id.clone())),
            Some(ApprovalState::Approved { effects, token, .. }) => {
                (effects.clone(), token.clone())
            }
            Some(other) => {
                return Err(WalletError::InvalidState {
                    request_id: request_id.clone(),
                    expected: "Approved".to_owned(),
                    actual: other.name().to_owned(),
                })
            }
        };
        let request = self.records[request_id].request.clone();

        let tx = match self.signer.sign(request_id, &request, &effects, &token) {
            Ok(tx) => tx,
            Err(err) => {
                self.fail(request_id, format!("signing failed: {}", err));
                return Err(err);
            }
        };
        if let Some(record) = self.records.get_mut(request_id) {
            record.state = ApprovalState::Signed {
                effects,
                token,
                tx: tx.clone(),
            };
        }
        let receipt = match self.signer.submit(&tx) {
            Ok(receipt) => receipt,
            Err(err) => {
                self.fail(request_id, format!("submission failed: {}", err));
                return Err(err);
            }
        };
        if let Some(record) = self.records.get_mut(request_id) {
            record.state = ApprovalState::Submitted { tx };
        }
        Ok(receipt)
    }

    fn record(&self, request_id: &RequestId) -> Result<&ApprovalRecord, WalletError> {
        self.records
            .get(request_id)
            .ok_or_else(|| WalletError::UnknownRequest(request_id.clone()))
    }

    fn fail(&mut self, request_id: &RequestId, reason: String) {
        if let Some(record) = self.records.get_mut(request_id) {
            record.state = ApprovalState::Failed { reason };
        }
    }
}

fn build_effects(
    request: &TransactionRequest,
    meta: Option<&ProgramMeta>,
    verification_status: VerificationStatus,
    sim: SimulationResult,
    mut warnings: Vec<String>,
) -> EffectsSummary {
    if sim.unlimited_approval && !warnings.iter().any(|w| w.contains("unlimited")) {
        warnings.push("transaction grants an UNLIMITED token approval".to_owned());
    }
    if sim.applies == Some(false) {
        warnings.push("simulation indicates the call will revert".to_owned());
    }
    let asset = sim.asset.clone().or_else(|| request.value.clone());
    let amount = asset.as_ref().map(|a| a.amount);
    EffectsSummary {
        requesting_dapp: request.dapp_id.clone(),
        session: request.session.clone(),
        target_account: request.fee_payer.clone(),
        target_zone: ZoneId::new("lez-testnet-0.4"),
        program_id: request.program_id.clone(),
        program_name: meta.map(|m| m.name.clone()),
        program_version: meta.map(|m| m.version.clone()),
        program_author: meta.map(|m| m.author.clone()),
        asset,
        amount,
        destination: sim.destination,
        approvals_granted: sim.approvals_granted,
        approvals_moved: sim.approvals_moved,
        unlimited_approval: sim.unlimited_approval,
        gas_estimate: sim.gas_estimate,
        gas_used: sim.gas_used,
        verification_status,
        verification_detail: meta.and_then(|m| m.code_pointer.clone()),
        warnings,
        raw_calldata_hex: hexutil::encode(&request.message),
        effects_available: sim.effects_available,
        actors_declared: if sim.effects_available {
            sim.actors_declared
        } else {
            request.declared_actors.clone()
        },
        signers_required: sim.signers_required,
        applies: sim.applies,
        fee_estimate: sim.fee_estimate,
    }
}

// ---------------------------------------------------------------------------
// Mocks. They are compiled into the library so downstream tests can reuse them.
// These are NOT chain/testnet behaviour: no network calls are made.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MockRegistry {
    programs: HashMap<ProgramId, (ProgramMeta, VerificationStatus)>,
    fail_resolve: bool,
}

impl Default for MockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MockRegistry {
    pub fn new() -> Self {
        Self {
            programs: HashMap::new(),
            fail_resolve: false,
        }
    }

    pub fn with_program(mut self, meta: ProgramMeta, status: VerificationStatus) -> Self {
        self.programs
            .insert(meta.program_id.clone(), (meta, status));
        self
    }

    pub fn failing(self) -> Self {
        Self {
            fail_resolve: true,
            ..self
        }
    }
}

impl ProgramRegistry for MockRegistry {
    fn resolve(&self, program_id: &ProgramId) -> Result<ProgramMeta, RegistryError> {
        if self.fail_resolve {
            return Err(RegistryError::Unavailable(
                "mock registry offline".to_owned(),
            ));
        }
        self.programs
            .get(program_id)
            .map(|(meta, _)| meta.clone())
            .ok_or_else(|| RegistryError::NotFound(program_id.clone()))
    }

    fn verification_status(&self, program_id: &ProgramId) -> VerificationStatus {
        self.programs
            .get(program_id)
            .map(|(_, status)| status.clone())
            .unwrap_or(VerificationStatus::Unregistered)
    }
}

#[derive(Debug, Clone)]
pub struct MockIdl {
    results: HashMap<ProgramId, SimulationResult>,
    fail: bool,
}

impl Default for MockIdl {
    fn default() -> Self {
        Self::new()
    }
}

impl MockIdl {
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
            fail: false,
        }
    }

    pub fn with_result(mut self, program_id: ProgramId, result: SimulationResult) -> Self {
        self.results.insert(program_id, result);
        self
    }

    pub fn failing(self) -> Self {
        Self { fail: true, ..self }
    }
}

impl IdlProvider for MockIdl {
    fn decode_and_simulate(
        &self,
        request: &TransactionRequest,
        _meta: Option<&ProgramMeta>,
    ) -> Result<SimulationResult, IdlError> {
        if self.fail {
            return Err(IdlError::Unavailable(request.program_id.clone()));
        }
        self.results
            .get(&request.program_id)
            .cloned()
            .ok_or_else(|| IdlError::Unavailable(request.program_id.clone()))
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuthorizerLog {
    pub calls: usize,
    pub grants_used: usize,
}

/// Mock core authorizer: only sessions/accounts explicitly granted by the
/// "user" are accepted. With no grants it is a deny-all authorizer.
#[derive(Debug, Clone)]
pub struct MockAuthorizer {
    grants: Rc<RefCell<HashSet<(SessionId, AccountId)>>>,
    log: Rc<RefCell<AuthorizerLog>>,
}

impl Default for MockAuthorizer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAuthorizer {
    pub fn new() -> Self {
        Self {
            grants: Rc::new(RefCell::new(HashSet::new())),
            log: Rc::new(RefCell::new(AuthorizerLog::default())),
        }
    }

    pub fn grant(&self, session: &SessionId, account: &AccountId) {
        self.grants
            .borrow_mut()
            .insert((session.clone(), account.clone()));
    }

    pub fn log_snapshot(&self) -> AuthorizerLog {
        self.log.borrow().clone()
    }
}

impl CoreAuthorizer for MockAuthorizer {
    fn approve(
        &self,
        session: &SessionId,
        account: &AccountId,
        _effects: &EffectsSummary,
    ) -> Result<ApprovalToken, Denied> {
        self.log.borrow_mut().calls += 1;
        if !self
            .grants
            .borrow()
            .contains(&(session.clone(), account.clone()))
        {
            return Err(Denied::UserRejected);
        }
        self.log.borrow_mut().grants_used += 1;
        Ok(ApprovalToken::new(format!(
            "token-{}",
            self.log.borrow().calls
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MockSignerLog {
    pub sign_calls: usize,
    pub submit_calls: usize,
}

#[derive(Debug, Clone)]
pub struct MockSigner {
    log: Rc<RefCell<MockSignerLog>>,
    fail_submit: bool,
}

impl Default for MockSigner {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSigner {
    pub fn new() -> Self {
        Self {
            log: Rc::new(RefCell::new(MockSignerLog::default())),
            fail_submit: false,
        }
    }

    pub fn failing_submit(self) -> Self {
        Self {
            fail_submit: true,
            ..self
        }
    }

    pub fn log_snapshot(&self) -> MockSignerLog {
        self.log.borrow().clone()
    }
}

impl Signer for MockSigner {
    fn sign(
        &self,
        request_id: &RequestId,
        request: &TransactionRequest,
        _effects: &EffectsSummary,
        token: &ApprovalToken,
    ) -> Result<SignedTransaction, WalletError> {
        self.log.borrow_mut().sign_calls += 1;
        Ok(SignedTransaction {
            request_id: request_id.clone(),
            account: request.fee_payer.clone(),
            signature: format!("mock-signature-for-{}", token).into_bytes(),
            tx_bytes: request.message.clone(),
        })
    }

    fn submit(&self, tx: &SignedTransaction) -> Result<SubmissionReceipt, WalletError> {
        self.log.borrow_mut().submit_calls += 1;
        if self.fail_submit {
            return Err(WalletError::Signer("mock submit failure".to_owned()));
        }
        Ok(SubmissionReceipt {
            request_id: tx.request_id.clone(),
            accepted: true,
            signature_hex: hexutil::encode(&tx.signature),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wallet_types::{ApprovalGrant, Asset, AssetMovement, DappId, Mint};

    fn sample_request() -> TransactionRequest {
        TransactionRequest {
            dapp_id: DappId::new("testimonial-app"),
            session: SessionId::new("session-1"),
            program_id: ProgramId::new("testimonial-store"),
            message: vec![0xca, 0xfe, 0xba, 0xbe],
            declared_actors: vec![],
            fee_payer: AccountId::new("alice"),
            value: None,
        }
    }

    fn sample_meta() -> ProgramMeta {
        ProgramMeta {
            program_id: ProgramId::new("testimonial-store"),
            name: "testimonial_store".to_owned(),
            version: "0.1.0".to_owned(),
            author: "logos".to_owned(),
            description: "stores testimonials".to_owned(),
            idl_pointer: Some("idl://testimonial_store".to_owned()),
            code_pointer: Some("git+https://example.invalid/testimonial#abc123".to_owned()),
            registered_at: Some("2026-11-19".to_owned()),
            deployer_signed: true,
        }
    }

    fn sample_sim() -> SimulationResult {
        SimulationResult {
            effects_available: true,
            asset: Some(Asset {
                program_id: ProgramId::new("token-program"),
                mint: Mint::new("mint-lp"),
                amount: 500,
                decimals: 6,
            }),
            destination: Some("bob".to_owned()),
            approvals_granted: vec![ApprovalGrant {
                owner: AccountId::new("alice"),
                spender: "token-program".to_owned(),
                mint: Mint::new("mint-lp"),
                amount: None,
                unlimited: true,
            }],
            approvals_moved: vec![AssetMovement {
                from: "alice".to_owned(),
                to: "bob".to_owned(),
                program_id: ProgramId::new("token-program"),
                mint: Mint::new("mint-lp"),
                amount: 500,
            }],
            unlimited_approval: true,
            gas_estimate: Some(123),
            gas_used: Some(120),
            actors_declared: vec![],
            signers_required: vec![AccountId::new("alice")],
            applies: Some(true),
            fee_estimate: Some(7),
            notes: vec![],
        }
    }

    struct Fixture {
        core: WalletCore,
        signer: MockSigner,
        authorizer: MockAuthorizer,
        registry: MockRegistry,
        idl: MockIdl,
    }

    fn fixture() -> Fixture {
        let registry = MockRegistry::new().with_program(
            sample_meta(),
            VerificationStatus::LocallyVerified {
                source_commit: "abc123".to_owned(),
                image_id: "image-abc123".to_owned(),
            },
        );
        let idl = MockIdl::new().with_result(ProgramId::new("testimonial-store"), sample_sim());
        let signer = MockSigner::new();
        let authorizer = MockAuthorizer::new();
        let core = WalletCore::new(
            Box::new(registry.clone()),
            Box::new(idl.clone()),
            Box::new(authorizer.clone()),
            Box::new(signer.clone()),
        );
        Fixture {
            core,
            signer,
            authorizer,
            registry,
            idl,
        }
    }

    fn approve(f: &mut Fixture, id: &RequestId) -> ApprovalToken {
        f.authorizer
            .grant(&SessionId::new("session-1"), &AccountId::new("alice"));
        f.core
            .authorize(
                id,
                &SessionId::new("session-1"),
                &AccountId::new("alice"),
                Capability::ProposeTransaction,
            )
            .unwrap()
    }

    #[test]
    fn propose_builds_human_readable_effects_and_waits() {
        let mut f = fixture();
        let id = f.core.propose_transaction(sample_request()).unwrap();
        assert_eq!(f.core.state(&id).unwrap().name(), "AwaitingApproval");
        let effects = f.core.effects_preview(&id).unwrap();
        assert_eq!(effects.program_name.as_deref(), Some("testimonial_store"));
        assert_eq!(
            effects.verification_status,
            VerificationStatus::LocallyVerified {
                source_commit: "abc123".into(),
                image_id: "image-abc123".into(),
            }
        );
        assert!(effects.effects_available);
        assert_eq!(effects.raw_calldata_hex, "cafebabe");
        let lines = effects.to_human_lines();
        assert!(lines.iter().any(|l| l.contains("Destination: bob")));
        assert!(lines.iter().any(|l| l.contains("UNLIMITED APPROVAL")));
        assert_eq!(f.signer.log_snapshot(), MockSignerLog::default());
    }

    #[test]
    fn negative_sign_without_explicit_authorize_is_blocked() {
        let mut f = fixture();
        let id = f.core.propose_transaction(sample_request()).unwrap();
        let before = f.signer.log_snapshot();

        let err = f.core.sign_and_submit(&id).unwrap_err();
        assert!(matches!(
            err,
            WalletError::InvalidState { .. } | WalletError::NotApproved(_)
        ));
        assert_eq!(f.signer.log_snapshot(), before);
        assert_eq!(f.signer.log_snapshot().sign_calls, 0);
        assert_eq!(f.signer.log_snapshot().submit_calls, 0);
        assert_eq!(f.authorizer.log_snapshot().calls, 0);
        assert_eq!(f.core.state(&id).unwrap().name(), "AwaitingApproval");
    }

    #[test]
    fn negative_authorizer_denial_keeps_request_unapproved() {
        let mut f = fixture();
        let id = f.core.propose_transaction(sample_request()).unwrap();
        // No grant is configured: the core authorizer denies.
        assert_eq!(
            f.core.authorize(
                &id,
                &SessionId::new("session-1"),
                &AccountId::new("alice"),
                Capability::ProposeTransaction
            ),
            Err(Denied::UserRejected)
        );
        assert_eq!(f.core.state(&id).unwrap().name(), "AwaitingApproval");
        assert!(f
            .core
            .sign_and_submit(&id)
            .unwrap_err()
            .to_string()
            .contains("AwaitingApproval"));
        assert_eq!(f.signer.log_snapshot().sign_calls, 0);
    }

    #[test]
    fn negative_failed_registry_and_idl_degrades_without_silent_approval() {
        let mut f = fixture();
        f.registry = MockRegistry::new().failing();
        f.idl = MockIdl::new().failing();
        f.core = WalletCore::new(
            Box::new(f.registry.clone()),
            Box::new(f.idl.clone()),
            Box::new(f.authorizer.clone()),
            Box::new(f.signer.clone()),
        );

        let id = f.core.propose_transaction(sample_request()).unwrap();
        let effects = f.core.effects_preview(&id).unwrap();
        assert!(!effects.effects_available);
        assert_eq!(
            effects.verification_status,
            VerificationStatus::Unregistered
        );
        assert!(effects
            .warnings
            .iter()
            .any(|w| w.contains("registry lookup failed")));
        assert!(effects
            .warnings
            .iter()
            .any(|w| w.contains("cannot render effects")));
        assert!(effects
            .to_human_lines()
            .iter()
            .any(|l| l.contains("raw calldata only")));
        assert_eq!(effects.raw_calldata_hex, "cafebabe");
        assert_eq!(f.core.state(&id).unwrap().name(), "AwaitingApproval");

        // Degradation must not turn into approval: signing is still blocked.
        assert!(f.core.sign_and_submit(&id).is_err());
        assert_eq!(f.signer.log_snapshot(), MockSignerLog::default());

        // Only an explicit authorize() unlocks signing, with warnings intact.
        let _ = approve(&mut f, &id);
        assert_eq!(f.core.state(&id).unwrap().name(), "Approved");
        assert!(f
            .core
            .effects_preview(&id)
            .unwrap()
            .warnings
            .iter()
            .any(|w| w.contains("IDL unavailable")));
        assert!(f.core.sign_and_submit(&id).is_ok());
        assert_eq!(f.signer.log_snapshot().sign_calls, 1);
        assert_eq!(f.signer.log_snapshot().submit_calls, 1);
    }

    #[test]
    fn unregistered_program_is_not_silently_approved() {
        let mut f = fixture();
        f.registry = MockRegistry::new(); // no entry
        f.core = WalletCore::new(
            Box::new(f.registry.clone()),
            Box::new(MockIdl::new().with_result(ProgramId::new("testimonial-store"), sample_sim())),
            Box::new(f.authorizer.clone()),
            Box::new(f.signer.clone()),
        );
        let id = f.core.propose_transaction(sample_request()).unwrap();
        let effects = f.core.effects_preview(&id).unwrap();
        assert_eq!(
            effects.verification_status,
            VerificationStatus::Unregistered
        );
        assert_eq!(effects.program_name, None);
        assert!(f.core.sign_and_submit(&id).is_err());
        assert_eq!(f.signer.log_snapshot().sign_calls, 0);
    }

    #[test]
    fn mismatch_verification_is_flagged_in_the_prompt() {
        let mut f = fixture();
        f.registry = MockRegistry::new().with_program(sample_meta(), VerificationStatus::Mismatch);
        f.core = WalletCore::new(
            Box::new(f.registry.clone()),
            Box::new(MockIdl::new().with_result(ProgramId::new("testimonial-store"), sample_sim())),
            Box::new(f.authorizer.clone()),
            Box::new(f.signer.clone()),
        );
        let id = f.core.propose_transaction(sample_request()).unwrap();
        let effects = f.core.effects_preview(&id).unwrap();
        assert!(effects
            .warnings
            .iter()
            .any(|w| w.contains("source image does not match")));
        assert!(effects
            .to_human_lines()
            .iter()
            .any(|l| l.contains("source mismatch")));
    }

    #[test]
    fn happy_path_authorize_then_sign_and_submit() {
        let mut f = fixture();
        let id = f.core.propose_transaction(sample_request()).unwrap();
        let token = approve(&mut f, &id);
        assert_eq!(f.core.state(&id).unwrap().name(), "Approved");
        let receipt = f.core.sign_and_submit(&id).unwrap();
        assert!(receipt.accepted);
        assert_eq!(receipt.request_id, id);
        assert_eq!(f.core.state(&id).unwrap().name(), "Submitted");
        assert_eq!(
            f.signer.log_snapshot(),
            MockSignerLog {
                sign_calls: 1,
                submit_calls: 1
            }
        );
        assert_eq!(f.authorizer.log_snapshot().grants_used, 1);
        assert_eq!(token, ApprovalToken::new("token-1"));
    }

    #[test]
    fn rejected_request_can_never_be_signed() {
        let mut f = fixture();
        let id = f.core.propose_transaction(sample_request()).unwrap();
        f.core
            .reject(&id, Some("user changed mind".into()))
            .unwrap();
        assert_eq!(f.core.state(&id).unwrap().name(), "Rejected");
        assert!(f
            .core
            .authorize(
                &id,
                &SessionId::new("session-1"),
                &AccountId::new("alice"),
                Capability::ProposeTransaction
            )
            .is_err());
        assert!(f.core.sign_and_submit(&id).is_err());
        assert_eq!(f.signer.log_snapshot().sign_calls, 0);
    }

    #[test]
    fn wrong_account_session_or_capability_is_denied() {
        let mut f = fixture();
        let id = f.core.propose_transaction(sample_request()).unwrap();
        f.authorizer
            .grant(&SessionId::new("session-1"), &AccountId::new("alice"));
        assert_eq!(
            f.core.authorize(
                &id,
                &SessionId::new("session-1"),
                &AccountId::new("bob"),
                Capability::ProposeTransaction
            ),
            Err(Denied::AccountMismatch)
        );
        assert_eq!(
            f.core.authorize(
                &id,
                &SessionId::new("other"),
                &AccountId::new("alice"),
                Capability::ProposeTransaction
            ),
            Err(Denied::NoSession)
        );
        assert_eq!(
            f.core.authorize(
                &id,
                &SessionId::new("session-1"),
                &AccountId::new("alice"),
                Capability::ReadBalances
            ),
            Err(Denied::CapabilityNotGranted(Capability::ReadBalances))
        );
        assert_eq!(f.core.state(&id).unwrap().name(), "AwaitingApproval");
    }

    #[test]
    fn submit_failure_marks_state_failed() {
        let mut f = fixture();
        f.signer = MockSigner::new().failing_submit();
        f.core = WalletCore::new(
            Box::new(f.registry.clone()),
            Box::new(f.idl.clone()),
            Box::new(f.authorizer.clone()),
            Box::new(f.signer.clone()),
        );
        let id = f.core.propose_transaction(sample_request()).unwrap();
        let _ = approve(&mut f, &id);
        assert!(f.core.sign_and_submit(&id).is_err());
        assert_eq!(f.core.state(&id).unwrap().name(), "Failed");
        assert_eq!(f.signer.log_snapshot().sign_calls, 1);
        assert_eq!(f.signer.log_snapshot().submit_calls, 1);
    }

    #[test]
    fn unknown_request_and_duplicate_approval_are_errors() {
        let mut f = fixture();
        let unknown = RequestId::new("req-does-not-exist");
        assert_eq!(
            f.core.state(&unknown),
            Err(WalletError::UnknownRequest(unknown.clone()))
        );
        assert!(f.core.sign_and_submit(&unknown).is_err());
        let id = f.core.propose_transaction(sample_request()).unwrap();
        let _ = approve(&mut f, &id);
        assert!(f
            .core
            .authorize(
                &id,
                &SessionId::new("session-1"),
                &AccountId::new("alice"),
                Capability::ProposeTransaction
            )
            .is_err());
        // Already approved: a second signing pass cannot reuse the token.
        let _ = f.core.sign_and_submit(&id);
        assert!(f.core.sign_and_submit(&id).is_err());
        assert_eq!(f.signer.log_snapshot().sign_calls, 1);
    }

    #[test]
    fn request_ids_are_unique() {
        let mut f = fixture();
        let a = f.core.propose_transaction(sample_request()).unwrap();
        let b = f.core.propose_transaction(sample_request()).unwrap();
        assert_ne!(a, b);
    }
}
