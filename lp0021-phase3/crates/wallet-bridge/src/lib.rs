// SPDX-License-Identifier: MIT AND Apache-2.0
//! Adapter layer for the LP-0021 phase 3 local asset model.
//!
//! HONESTY: the "token program" here is a Rust in-memory adapter over
//! `wallet-assets`. There is no on-chain program, no LEZ call, no signature
//! and no network I/O. [`UnavailableTokenProgram`] exists so callers can see
//! the explicit no-backend case instead of a silent fake success.
//!
//! [`PrivateAccountRegistry`] translates opaque **non-cryptographic** handles
//! into accounts. Neither its errors nor its log output ever include a handle.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

use wallet_assets::{
    Account, AccountId, AccountKind, Allowance, ApprovalId, ApprovalRequest, AssetId, LedgerError,
    MintId, PrivateHandle, PrivateTransfer, Receipt, TokenLedger, TokenMeta, Transfer,
};

/// Errors surfaced by token-program adapters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeError {
    /// The local ledger refused the operation.
    Ledger(LedgerError),
    /// A private-handle registry refused the operation.
    Registry(RegistryError),
    /// Mint metadata could not be resolved.
    Discovery(DiscoveryError),
    /// No token program backend exists in this environment.
    Unavailable,
}

impl Display for BridgeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ledger(error) => write!(f, "local ledger error: {error}"),
            Self::Registry(error) => write!(f, "private account registry error: {error}"),
            Self::Discovery(error) => write!(f, "mint discovery error: {error}"),
            Self::Unavailable => write!(f, "token program adapter unavailable"),
        }
    }
}

impl std::error::Error for BridgeError {}

impl From<LedgerError> for BridgeError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}

impl From<RegistryError> for BridgeError {
    fn from(error: RegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<DiscoveryError> for BridgeError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

/// Adapter contract for a fungible-token backend.
///
/// This trait deliberately mirrors only local operations. An implementation
/// backed by a real token program does not exist here.
pub trait TokenProgram {
    /// Create a simulated account.
    fn create_account(&mut self, kind: AccountKind) -> Result<Account, BridgeError>;

    /// Create a simulated private account and register its opaque handle.
    fn create_private_account(&mut self) -> Result<(Account, PrivateHandle), BridgeError>;

    /// Resolve an opaque handle without exposing it in errors.
    fn resolve_private_handle(&self, handle: PrivateHandle) -> Result<Account, BridgeError>;

    /// Register mint metadata locally.
    fn register_token(&mut self, meta: TokenMeta) -> Result<(), BridgeError>;

    /// Local mint credit.
    fn mint(&mut self, to: Account, asset: AssetId, amount: u128) -> Result<(), BridgeError>;

    /// Local burn debit.
    fn burn(&mut self, from: Account, asset: AssetId, amount: u128) -> Result<(), BridgeError>;

    /// Create an explicit approval record.
    fn approve(&mut self, request: ApprovalRequest) -> Result<ApprovalId, BridgeError>;

    /// Public-to-public transfer.
    fn transfer_public(&mut self, tx: Transfer) -> Result<Receipt, BridgeError>;

    /// Private-to-private transfer using opaque handles.
    fn transfer_private(&mut self, tx: PrivateTransfer) -> Result<Receipt, BridgeError>;

    /// Explicitly approved public/private transfer.
    fn transfer_cross(&mut self, tx: Transfer) -> Result<Receipt, BridgeError>;

    /// Per-account, per-asset balance query.
    fn balance(&self, account: Account, asset: AssetId) -> Result<u128, BridgeError>;

    /// Aggregate one asset across accounts.
    fn aggregate(&self, accounts: &[Account], asset: AssetId) -> Result<u128, BridgeError>;

    /// Current allowance for an approval.
    fn allowance(&self, id: ApprovalId) -> Result<Allowance, BridgeError>;
}

/// Token-program adapter backed only by the local `TokenLedger`.
#[derive(Clone, Debug, Default)]
pub struct InMemoryTokenProgram {
    ledger: TokenLedger,
    registry: PrivateAccountRegistry,
}

impl InMemoryTokenProgram {
    /// Empty adapter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Read-only access to the local ledger.
    pub fn ledger(&self) -> &TokenLedger {
        &self.ledger
    }

    /// Mutable access to the local ledger.
    pub fn ledger_mut(&mut self) -> &mut TokenLedger {
        &mut self.ledger
    }

    /// Read-only access to the private-handle registry.
    pub fn registry(&self) -> &PrivateAccountRegistry {
        &self.registry
    }
}

impl TokenProgram for InMemoryTokenProgram {
    fn create_account(&mut self, kind: AccountKind) -> Result<Account, BridgeError> {
        Ok(self.ledger.create_account(kind)?)
    }

    fn create_private_account(&mut self) -> Result<(Account, PrivateHandle), BridgeError> {
        let account = self.ledger.create_account(AccountKind::Private)?;
        let handle = self.ledger.private_handle(account)?;
        self.registry.insert(handle, account)?;
        Ok((account, handle))
    }

    fn resolve_private_handle(&self, handle: PrivateHandle) -> Result<Account, BridgeError> {
        Ok(self.registry.resolve(handle)?)
    }

    fn register_token(&mut self, meta: TokenMeta) -> Result<(), BridgeError> {
        Ok(self.ledger.register_token(meta)?)
    }

    fn mint(&mut self, to: Account, asset: AssetId, amount: u128) -> Result<(), BridgeError> {
        Ok(self.ledger.mint(to, asset, amount)?)
    }

    fn burn(&mut self, from: Account, asset: AssetId, amount: u128) -> Result<(), BridgeError> {
        Ok(self.ledger.burn(from, asset, amount)?)
    }

    fn approve(&mut self, request: ApprovalRequest) -> Result<ApprovalId, BridgeError> {
        Ok(self.ledger.approve(request)?)
    }

    fn transfer_public(&mut self, tx: Transfer) -> Result<Receipt, BridgeError> {
        Ok(self.ledger.transfer_public(tx)?)
    }

    fn transfer_private(&mut self, tx: PrivateTransfer) -> Result<Receipt, BridgeError> {
        let from = self.registry.resolve(tx.from_handle)?;
        let to = self.registry.resolve(tx.to_handle)?;
        let ledger_from = self.ledger.resolve_handle(tx.from_handle)?;
        let ledger_to = self.ledger.resolve_handle(tx.to_handle)?;
        if from != ledger_from || to != ledger_to {
            return Err(BridgeError::Registry(RegistryError::UnknownHandle));
        }
        Ok(self.ledger.transfer_private(tx)?)
    }

    fn transfer_cross(&mut self, tx: Transfer) -> Result<Receipt, BridgeError> {
        Ok(self.ledger.transfer_cross(tx)?)
    }

    fn balance(&self, account: Account, asset: AssetId) -> Result<u128, BridgeError> {
        Ok(self.ledger.balance(account, asset)?)
    }

    fn aggregate(&self, accounts: &[Account], asset: AssetId) -> Result<u128, BridgeError> {
        Ok(self.ledger.aggregate(accounts, asset)?)
    }

    fn allowance(&self, id: ApprovalId) -> Result<Allowance, BridgeError> {
        Ok(self.ledger.allowance(id)?)
    }
}

/// Explicit "nothing is connected" implementation. Every call fails loudly.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableTokenProgram;

impl TokenProgram for UnavailableTokenProgram {
    fn create_account(&mut self, _kind: AccountKind) -> Result<Account, BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn create_private_account(&mut self) -> Result<(Account, PrivateHandle), BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn resolve_private_handle(&self, _handle: PrivateHandle) -> Result<Account, BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn register_token(&mut self, _meta: TokenMeta) -> Result<(), BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn mint(&mut self, _to: Account, _asset: AssetId, _amount: u128) -> Result<(), BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn burn(&mut self, _from: Account, _asset: AssetId, _amount: u128) -> Result<(), BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn approve(&mut self, _request: ApprovalRequest) -> Result<ApprovalId, BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn transfer_public(&mut self, _tx: Transfer) -> Result<Receipt, BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn transfer_private(&mut self, _tx: PrivateTransfer) -> Result<Receipt, BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn transfer_cross(&mut self, _tx: Transfer) -> Result<Receipt, BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn balance(&self, _account: Account, _asset: AssetId) -> Result<u128, BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn aggregate(&self, _accounts: &[Account], _asset: AssetId) -> Result<u128, BridgeError> {
        Err(BridgeError::Unavailable)
    }

    fn allowance(&self, _id: ApprovalId) -> Result<Allowance, BridgeError> {
        Err(BridgeError::Unavailable)
    }
}

/// Mint metadata resolution adapter.
pub trait MintDiscovery {
    /// Resolve metadata for a mint id.
    fn resolve(&self, mint: MintId) -> Result<TokenMeta, DiscoveryError>;

    /// Stable name of the discovery backend.
    fn source(&self) -> &'static str;
}

/// Errors from mint metadata discovery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscoveryError {
    /// Metadata for this mint is not present locally.
    UnknownMint(MintId),
    /// This mint was registered twice.
    DuplicateMint(MintId),
    /// No discovery backend is available.
    Unavailable,
}

impl Display for DiscoveryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownMint(mint) => write!(f, "mint metadata not found: {mint}"),
            Self::DuplicateMint(mint) => write!(f, "mint metadata already present: {mint}"),
            Self::Unavailable => write!(f, "mint discovery unavailable"),
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// Local metadata store.
#[derive(Clone, Debug, Default)]
pub struct InMemoryMintDiscovery {
    tokens: BTreeMap<MintId, TokenMeta>,
}

impl InMemoryMintDiscovery {
    /// Empty discovery store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert metadata. Duplicate mints are rejected.
    pub fn insert(&mut self, meta: TokenMeta) -> Result<(), DiscoveryError> {
        let mint = meta.mint();
        if self.tokens.contains_key(&mint) {
            return Err(DiscoveryError::DuplicateMint(mint));
        }
        self.tokens.insert(mint, meta);
        Ok(())
    }

    /// Number of known mints.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Whether no mints are known.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

impl MintDiscovery for InMemoryMintDiscovery {
    fn resolve(&self, mint: MintId) -> Result<TokenMeta, DiscoveryError> {
        self.tokens
            .get(&mint)
            .cloned()
            .ok_or(DiscoveryError::UnknownMint(mint))
    }

    fn source(&self) -> &'static str {
        "in-memory"
    }
}

/// Explicit no-discovery implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableMintDiscovery;

impl MintDiscovery for UnavailableMintDiscovery {
    fn resolve(&self, _mint: MintId) -> Result<TokenMeta, DiscoveryError> {
        Err(DiscoveryError::Unavailable)
    }

    fn source(&self) -> &'static str {
        "unavailable"
    }
}

/// Errors from the opaque-handle registry. None of these carry a handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// No account is registered for this handle.
    UnknownHandle,
    /// This handle was already registered.
    DuplicateHandle,
    /// The account is not a simulated private account.
    NotPrivateAccount(AccountId),
}

impl Display for RegistryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownHandle => write!(f, "unknown opaque private account handle"),
            Self::DuplicateHandle => write!(f, "opaque private account handle already registered"),
            Self::NotPrivateAccount(account) => {
                write!(f, "account {account} is not a private account")
            }
        }
    }
}

impl std::error::Error for RegistryError {}

/// Maps opaque non-crypto private handles to accounts without ever logging,
/// formatting or returning the handle in an error.
#[derive(Clone, Debug, Default)]
pub struct PrivateAccountRegistry {
    by_handle: BTreeMap<PrivateHandle, Account>,
}

impl PrivateAccountRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handle for a simulated private account.
    pub fn insert(&mut self, handle: PrivateHandle, account: Account) -> Result<(), RegistryError> {
        if account.kind() != AccountKind::Private {
            return Err(RegistryError::NotPrivateAccount(account.id()));
        }
        if self.by_handle.contains_key(&handle) {
            return Err(RegistryError::DuplicateHandle);
        }
        self.by_handle.insert(handle, account);
        Ok(())
    }

    /// Resolve a handle to an account. Errors never contain the handle.
    pub fn resolve(&self, handle: PrivateHandle) -> Result<Account, RegistryError> {
        self.by_handle
            .get(&handle)
            .copied()
            .ok_or(RegistryError::UnknownHandle)
    }

    /// Whether a handle is known.
    pub fn contains(&self, handle: PrivateHandle) -> bool {
        self.by_handle.contains_key(&handle)
    }

    /// Number of registered handles.
    pub fn len(&self) -> usize {
        self.by_handle.len()
    }

    /// Whether no handles are registered.
    pub fn is_empty(&self) -> bool {
        self.by_handle.is_empty()
    }

    /// All registered accounts, in opaque-handle order.
    pub fn accounts(&self) -> Vec<Account> {
        self.by_handle.values().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wallet_assets::{AmountBound, ProgramId, Spender, TokenLedger, TransferShape};

    const MINT: MintId = MintId::new(7);

    fn token() -> AssetId {
        AssetId::mint(MINT)
    }

    fn funded(program: &mut InMemoryTokenProgram) -> (Account, Account, ApprovalId) {
        program
            .register_token(TokenMeta::new(MINT, 0, "TKN").unwrap())
            .unwrap();
        let alice = program.create_account(AccountKind::Public).unwrap();
        let (bob, _) = program.create_private_account().unwrap();
        program.mint(alice, token(), 100).unwrap();
        let approval = program
            .approve(ApprovalRequest {
                owner: alice,
                spender: Spender::Account(bob.id()),
                asset: token(),
                bound: AmountBound::limited(50),
                allow_cross_kind: true,
            })
            .unwrap();
        (alice, bob, approval)
    }

    #[test]
    fn in_memory_program_mints_and_queries_balance() {
        let mut program = InMemoryTokenProgram::new();
        let (alice, bob, _) = funded(&mut program);
        assert_eq!(program.balance(alice, token()), Ok(100));
        assert_eq!(program.balance(bob, token()), Ok(0));
        assert_eq!(
            program.allowance(ApprovalId::new(1)).unwrap().remaining(),
            50
        );
    }

    #[test]
    fn in_memory_program_reports_missing_approval() {
        let mut program = InMemoryTokenProgram::new();
        let (alice, bob, _) = funded(&mut program);
        assert_eq!(
            program.transfer_cross(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 1,
                nonce: 0,
                approval: ApprovalId::new(99),
            }),
            Err(BridgeError::Ledger(LedgerError::UnknownApproval(
                ApprovalId::new(99)
            )))
        );
    }

    #[test]
    fn in_memory_program_executes_approved_cross_transfer() {
        let mut program = InMemoryTokenProgram::new();
        let (alice, bob, approval) = funded(&mut program);
        let receipt = program
            .transfer_cross(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 8,
                nonce: 0,
                approval,
            })
            .unwrap();
        assert_eq!(receipt.nonce, 0);
        assert_eq!(program.balance(bob, token()), Ok(8));
        match program.ledger().events().last().unwrap() {
            wallet_assets::LedgerEvent::Transferred { shape, .. } => {
                assert_eq!(*shape, TransferShape::Cross)
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn aggregate_via_program_spans_public_and_private() {
        let mut program = InMemoryTokenProgram::new();
        let (alice, bob, approval) = funded(&mut program);
        program
            .transfer_cross(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 25,
                nonce: 0,
                approval,
            })
            .unwrap();
        assert_eq!(program.aggregate(&[alice, bob], token()), Ok(100));
    }

    #[test]
    fn unavailable_program_fails_every_operation_loudly() {
        let mut program = UnavailableTokenProgram;
        assert_eq!(
            program.create_account(AccountKind::Public),
            Err(BridgeError::Unavailable)
        );
        assert_eq!(
            program.balance(
                Account::new(AccountId::new(1), AccountKind::Public),
                token()
            ),
            Err(BridgeError::Unavailable)
        );
        assert_eq!(
            program.approve(ApprovalRequest {
                owner: Account::new(AccountId::new(1), AccountKind::Public),
                spender: Spender::Program(ProgramId::new("P").unwrap()),
                asset: token(),
                bound: AmountBound::unlimited(),
                allow_cross_kind: false,
            }),
            Err(BridgeError::Unavailable)
        );
        assert!(BridgeError::Unavailable.to_string().contains("unavailable"));
    }

    #[test]
    fn static_mint_discovery_resolves_metadata() {
        let mut discovery = InMemoryMintDiscovery::new();
        discovery
            .insert(TokenMeta::new(MINT, 2, "TKN").unwrap())
            .unwrap();
        assert_eq!(discovery.source(), "in-memory");
        assert_eq!(discovery.len(), 1);
        assert!(!discovery.is_empty());
        let meta = discovery.resolve(MINT).unwrap();
        assert_eq!(meta.symbol(), "TKN");
        assert_eq!(meta.decimals(), 2);
    }

    #[test]
    fn mint_discovery_reports_unknown_duplicate_and_unavailable() {
        let mut discovery = InMemoryMintDiscovery::new();
        assert_eq!(
            discovery.resolve(MINT),
            Err(DiscoveryError::UnknownMint(MINT))
        );
        discovery
            .insert(TokenMeta::new(MINT, 0, "TKN").unwrap())
            .unwrap();
        assert_eq!(
            discovery.insert(TokenMeta::new(MINT, 0, "TKN").unwrap()),
            Err(DiscoveryError::DuplicateMint(MINT))
        );
        assert_eq!(
            UnavailableMintDiscovery.resolve(MINT),
            Err(DiscoveryError::Unavailable)
        );
        assert_eq!(UnavailableMintDiscovery.source(), "unavailable");
    }

    #[test]
    fn registry_maps_opaque_handles_without_exposing_them() {
        let mut program = InMemoryTokenProgram::new();
        let (account, handle) = program.create_private_account().unwrap();
        assert!(program.registry().contains(handle));
        assert_eq!(program.resolve_private_handle(handle), Ok(account));
        assert_eq!(program.registry().accounts(), vec![account]);
        let rendered_handle = handle.to_string();
        let error = RegistryError::UnknownHandle.to_string();
        assert!(!error.contains(&rendered_handle));
        assert!(!error.contains("pv-handle"));
        assert!(!BridgeError::from(RegistryError::UnknownHandle)
            .to_string()
            .contains("pv-handle"));
    }

    #[test]
    fn registry_rejects_duplicates_non_private_and_unknown_handles() {
        let mut program = InMemoryTokenProgram::new();
        let (private, handle) = program.create_private_account().unwrap();
        let mut issuer = TokenLedger::new();
        let _other = issuer.create_account(AccountKind::Private).unwrap();
        let foreign = issuer.create_account(AccountKind::Private).unwrap();
        let foreign_handle = issuer.private_handle(foreign).unwrap();

        let mut registry = PrivateAccountRegistry::new();
        assert!(registry.is_empty());
        registry.insert(handle, private).unwrap();
        assert_eq!(
            registry.insert(handle, private),
            Err(RegistryError::DuplicateHandle)
        );
        assert_eq!(
            registry.resolve(foreign_handle),
            Err(RegistryError::UnknownHandle)
        );
        let public = Account::new(AccountId::new(1), AccountKind::Public);
        assert_eq!(
            registry.insert(foreign_handle, public),
            Err(RegistryError::NotPrivateAccount(public.id()))
        );
    }

    #[test]
    fn private_transfer_via_program_resolves_handles() {
        let mut program = InMemoryTokenProgram::new();
        program
            .register_token(TokenMeta::new(MINT, 0, "TKN").unwrap())
            .unwrap();
        let (alice, alice_handle) = program.create_private_account().unwrap();
        let (bob, bob_handle) = program.create_private_account().unwrap();
        program.mint(alice, token(), 30).unwrap();
        let approval = program
            .approve(ApprovalRequest {
                owner: alice,
                spender: Spender::Account(bob.id()),
                asset: token(),
                bound: AmountBound::limited(30),
                allow_cross_kind: false,
            })
            .unwrap();
        program
            .transfer_private(PrivateTransfer {
                from_handle: alice_handle,
                to_handle: bob_handle,
                asset: token(),
                amount: 11,
                nonce: 0,
                approval,
            })
            .unwrap();
        assert_eq!(program.balance(alice, token()), Ok(19));
        assert_eq!(program.balance(bob, token()), Ok(11));
    }
}
