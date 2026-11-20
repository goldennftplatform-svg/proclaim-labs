// SPDX-License-Identifier: MIT AND Apache-2.0
//! Dependency-free, in-memory permission and consent model for a wallet.
//!
//! This crate models the requirements of LP-0021 (F4/F5/R2) and LP-0001
//! (U5/U6/F10):
//!
//! * Capabilities are granted **individually**, never bundled.
//! * A grant is legible as *what it discloses*, not just as an API name.
//! * A dApp holding only [`Capability::ProofRequest`] can obtain a proof, but
//!   cannot learn an account address, balances, NFT holdings, or any token.
//! * Nothing is signed or submitted. Every on-chain-looking object is a mock.
//!
//! # Deny by default
//!
//! [`PermissionStore::has`] returns `false` unless an active, unexpired grant
//! matches the dApp, capability, and scope exactly.

#![forbid(unsafe_code)]

use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

/// Issuer namespaces are process-local counters, not cryptographic identities.
static NEXT_STORE_ISSUER: AtomicU64 = AtomicU64::new(1);

fn fresh_issuer() -> u64 {
    NEXT_STORE_ISSUER.fetch_add(1, Ordering::Relaxed)
}

/// A wallet permission. Each one is approved and revoked on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// Produce a proof about a statement, without revealing account data.
    ProofRequest,
    /// Learn which account the user chose to share.
    AccountAccess,
    /// Enumerate NFTs in a specific approved collection.
    NftEnumeration,
    /// Ask the user to approve a transfer; never signs or submits it.
    ProposeTransfer,
    /// Read the balance of a specific approved account.
    BalanceRead,
    /// Request a signature for a transaction the user explicitly approved.
    SignTransaction,
}

impl Capability {
    /// A plain-language label, distinct from the API name.
    pub fn human_label(&self) -> &'static str {
        match self {
            Capability::ProofRequest => "proof request",
            Capability::AccountAccess => "account access",
            Capability::NftEnumeration => "NFT enumeration",
            Capability::ProposeTransfer => "transaction proposal",
            Capability::BalanceRead => "balance read",
            Capability::SignTransaction => "transaction signature",
        }
    }
}

impl Display for Capability {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Capability::ProofRequest => "ProofRequest",
            Capability::AccountAccess => "AccountAccess",
            Capability::NftEnumeration => "NftEnumeration",
            Capability::ProposeTransfer => "ProposeTransfer",
            Capability::BalanceRead => "BalanceRead",
            Capability::SignTransaction => "SignTransaction",
        };
        write!(f, "{name}")
    }
}

/// What a grant discloses, written for a non-expert user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disclosure {
    reveals: Vec<String>,
    does_not_reveal: Vec<String>,
}

impl Disclosure {
    /// Build a disclosure from explicit plain-language statements.
    pub fn new(reveals: Vec<String>, does_not_reveal: Vec<String>) -> Self {
        Self {
            reveals,
            does_not_reveal,
        }
    }

    /// The canonical, store-owned text for a capability in a given scope.
    ///
    /// The store uses this rather than trusting caller-supplied text, so a
    /// grant can never be mislabelled as revealing less than it does.
    pub fn for_capability(capability: Capability, scope: &Scope) -> Self {
        let (reveals, does_not_reveal): (Vec<String>, Vec<String>) = match capability {
            Capability::ProofRequest => (
                vec!["only whether the statement you asked about is true or false".to_string()],
                vec![
                    "your account address".to_string(),
                    "your balances".to_string(),
                    "which NFTs you own".to_string(),
                    "any specific token or collection".to_string(),
                    "any ability to move or sign for your funds".to_string(),
                ],
            ),
            Capability::AccountAccess => (
                vec![
                    "which account you choose to share, and that this dApp may ask about it"
                        .to_string(),
                ],
                vec![
                    "your other accounts".to_string(),
                    "your balances".to_string(),
                    "your NFT holdings".to_string(),
                    "permission to move funds".to_string(),
                ],
            ),
            Capability::NftEnumeration => (
                vec!["the NFTs in the specific collection you approved".to_string()],
                vec![
                    "your account address unless you grant account access separately".to_string(),
                    "your balances".to_string(),
                    "NFTs in other collections".to_string(),
                    "your private keys".to_string(),
                ],
            ),
            Capability::ProposeTransfer => (
                vec!["a description of the transfer you are about to approve".to_string()],
                vec![
                    "your private keys".to_string(),
                    "a signed or submitted transaction".to_string(),
                    "any moved funds until you approve this exact request".to_string(),
                ],
            ),
            Capability::BalanceRead => (
                vec!["the balance of the specific account you approved".to_string()],
                vec![
                    "your other accounts".to_string(),
                    "your NFT holdings".to_string(),
                    "your private keys".to_string(),
                ],
            ),
            Capability::SignTransaction => (
                vec![
                    "that a signature was requested for a transaction you explicitly approved"
                        .to_string(),
                ],
                vec![
                    "your private keys".to_string(),
                    "any transaction you did not explicitly approve".to_string(),
                    "your balances directly".to_string(),
                ],
            ),
        };

        let scope_phrase = match scope {
            Scope::Unspecified => "no account or collection is named".to_string(),
            Scope::Account(account) => format!("the account {account}"),
            Scope::Collection(collection) => format!("the collection {collection}"),
        };

        let mut all_reveals = reveals;
        all_reveals.push(format!("the approved scope: {scope_phrase}"));

        Self {
            reveals: all_reveals,
            does_not_reveal,
        }
    }

    /// Plain-language statements of what is revealed.
    pub fn reveals(&self) -> &[String] {
        &self.reveals
    }

    /// Plain-language statements of what is not revealed.
    pub fn does_not_reveal(&self) -> &[String] {
        &self.does_not_reveal
    }

    /// Render the disclosure as indented lines for a permissions screen.
    pub fn to_human_lines(&self) -> Vec<String> {
        let mut lines = vec!["  Discloses:".to_string()];
        for item in &self.reveals {
            lines.push(format!("    - {item}"));
        }
        lines.push("  Does not disclose:".to_string());
        for item in &self.does_not_reveal {
            lines.push(format!("    - {item}"));
        }
        lines
    }
}

impl Display for Disclosure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_human_lines().join("\n"))
    }
}

/// An opaque account identifier supplied by the wallet UI.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountId(String);

impl AccountId {
    /// Create an account identifier. In a real wallet this would not be a
    /// caller-chosen string; here it is a mock handle.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The underlying mock label.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for AccountId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An opaque NFT collection identifier supplied by the wallet UI.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CollectionId(String);

impl CollectionId {
    /// Create a collection identifier (mock handle).
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The underlying mock label.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CollectionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The thing a capability applies to.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scope {
    /// No account or collection is named (used by proof requests).
    Unspecified,
    /// Scoped to one account.
    Account(AccountId),
    /// Scoped to one collection.
    Collection(CollectionId),
}

impl Display for Scope {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Scope::Unspecified => write!(f, "unspecified"),
            Scope::Account(account) => write!(f, "account {account}"),
            Scope::Collection(collection) => write!(f, "collection {collection}"),
        }
    }
}

/// An opaque dApp identity. Fields are private: a dApp cannot fabricate one.
///
/// A `DappId` is only issued by [`PermissionStore::register_dapp`] and is
/// scoped to the issuing store, so an id from another store is not recognized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DappId {
    serial: u64,
    issuer: u64,
}

impl Display for DappId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "dapp#{}", self.serial)
    }
}

/// An opaque handle for one individually revocable grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GrantId(u64);

impl GrantId {
    /// Numeric form, useful for logging.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Display for GrantId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "grant#{}", self.0)
    }
}

/// A single permission grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    id: GrantId,
    dapp: DappId,
    capability: Capability,
    scope: Scope,
    disclosure: Disclosure,
    issued_at: u64,
    expires_at: Option<u64>,
    active: bool,
}

impl Grant {
    /// The handle used to revoke this grant on its own.
    pub fn id(&self) -> GrantId {
        self.id
    }

    /// The dApp this grant belongs to.
    pub fn dapp(&self) -> &DappId {
        &self.dapp
    }

    /// The capability that was granted.
    pub fn capability(&self) -> Capability {
        self.capability
    }

    /// The scope the capability was granted in.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Plain-language disclosure attached to this grant.
    pub fn disclosure(&self) -> &Disclosure {
        &self.disclosure
    }

    /// Clock value at which the grant was issued.
    pub fn issued_at(&self) -> u64 {
        self.issued_at
    }

    /// Last clock value at which the grant is valid, if bounded.
    pub fn expires_at(&self) -> Option<u64> {
        self.expires_at
    }

    /// Whether the grant has not been revoked.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Whether the grant has expired at the given clock value.
    pub fn is_expired_at(&self, now: u64) -> bool {
        self.expires_at.is_some_and(|expires| now > expires)
    }
}

/// Errors returned by the store and coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The dApp id was not issued by this store (forged or from another store).
    UnrecognizedDapp,
    /// No active, unexpired grant covers this request. Deny by default.
    Denied,
    /// A grant for the capability exists, but not for the requested scope.
    ScopeMismatch,
    /// The grant expired at the current clock value.
    Expired,
    /// No grant has the given id.
    GrantNotFound,
    /// The grant was already revoked.
    AlreadyRevoked,
    /// Approvals cover exactly one capability; bundles are never allowed.
    BundledApprovalNotAllowed,
    /// No pending transfer has the given request id.
    UnknownRequest,
    /// This pending transfer was already authorized.
    AlreadyAuthorized,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            Error::UnrecognizedDapp => "dApp id is not recognized by this wallet",
            Error::Denied => "permission denied (no matching active grant)",
            Error::ScopeMismatch => "permission granted only for a different scope",
            Error::Expired => "grant has expired",
            Error::GrantNotFound => "no grant has that id",
            Error::AlreadyRevoked => "grant was already revoked",
            Error::BundledApprovalNotAllowed => {
                "approvals are single-capability; bundled approval is not allowed"
            }
            Error::UnknownRequest => "no pending transfer has that request id",
            Error::AlreadyAuthorized => "that request was already authorized",
        };
        write!(f, "{message}")
    }
}

impl std::error::Error for Error {}

/// An in-memory, deny-by-default set of individual grants.
#[derive(Debug)]
pub struct PermissionStore {
    issuer: u64,
    next_dapp: u64,
    next_grant: u64,
    clock: u64,
    grants: Vec<Grant>,
}

impl Default for PermissionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionStore {
    /// Create an empty store with an empty permission set.
    pub fn new() -> Self {
        Self {
            issuer: fresh_issuer(),
            next_dapp: 1,
            next_grant: 1,
            clock: 0,
            grants: Vec::new(),
        }
    }

    /// The current logical time, used for expiry checks.
    pub fn now(&self) -> u64 {
        self.clock
    }

    /// Advance (or set) the logical clock.
    pub fn set_now(&mut self, now: u64) {
        self.clock = now;
    }

    /// Register a dApp and return an opaque, store-scoped identity for it.
    pub fn register_dapp(&mut self, _label: impl Into<String>) -> DappId {
        let dapp = DappId {
            serial: self.next_dapp,
            issuer: self.issuer,
        };
        self.next_dapp += 1;
        dapp
    }

    /// Whether the id was issued by this store and is still in range.
    pub fn recognizes(&self, dapp: &DappId) -> bool {
        dapp.issuer == self.issuer && dapp.serial >= 1 && dapp.serial < self.next_dapp
    }

    /// Issue one grant. The disclosure is generated by the store, never by
    /// the caller, so a capability cannot be mislabelled.
    pub fn grant(
        &mut self,
        dapp: &DappId,
        capability: Capability,
        scope: &Scope,
        issued_at: u64,
        expires_at: Option<u64>,
    ) -> Result<GrantId, Error> {
        if !self.recognizes(dapp) {
            return Err(Error::UnrecognizedDapp);
        }
        let id = GrantId(self.next_grant);
        self.next_grant += 1;
        let disclosure = Disclosure::for_capability(capability, scope);
        self.grants.push(Grant {
            id,
            dapp: *dapp,
            capability,
            scope: scope.clone(),
            disclosure,
            issued_at,
            expires_at,
            active: true,
        });
        Ok(id)
    }

    /// Deny-by-default check at the store's current clock value.
    pub fn has(&self, dapp: &DappId, capability: Capability, scope: &Scope) -> bool {
        self.has_at(dapp, capability, scope, self.clock)
    }

    /// Deny-by-default check at an explicit clock value.
    pub fn has_at(&self, dapp: &DappId, capability: Capability, scope: &Scope, now: u64) -> bool {
        if !self.recognizes(dapp) {
            return false;
        }
        self.grants.iter().any(|grant| {
            grant.dapp == *dapp
                && grant.capability == capability
                && grant.scope == *scope
                && grant.active
                && !grant.is_expired_at(now)
        })
    }

    /// Whether the dApp holds this capability in any scope. Used only to
    /// distinguish a scope mismatch from a plain denial in error reporting.
    pub fn has_any_scope_at(&self, dapp: &DappId, capability: Capability, now: u64) -> bool {
        if !self.recognizes(dapp) {
            return false;
        }
        self.grants.iter().any(|grant| {
            grant.dapp == *dapp
                && grant.capability == capability
                && grant.active
                && !grant.is_expired_at(now)
        })
    }

    /// Look up a grant by id.
    pub fn get(&self, id: GrantId) -> Option<&Grant> {
        self.grants.iter().find(|grant| grant.id == id)
    }

    /// Revoke exactly one grant. Other capabilities are unaffected.
    pub fn revoke(&mut self, id: GrantId) -> Result<(), Error> {
        match self.grants.iter_mut().find(|grant| grant.id == id) {
            Some(grant) if grant.active => {
                grant.active = false;
                Ok(())
            }
            Some(_) => Err(Error::AlreadyRevoked),
            None => Err(Error::GrantNotFound),
        }
    }

    /// Revoke every active grant for one dApp; other dApps are unaffected.
    pub fn revoke_all(&mut self, dapp: &DappId) -> Result<usize, Error> {
        if !self.recognizes(dapp) {
            return Err(Error::UnrecognizedDapp);
        }
        let mut count = 0;
        for grant in &mut self.grants {
            if grant.dapp == *dapp && grant.active {
                grant.active = false;
                count += 1;
            }
        }
        Ok(count)
    }

    /// Deactivate every grant expired at `now`; return how many were removed.
    pub fn revoke_expired(&mut self, now: u64) -> usize {
        let mut count = 0;
        for grant in &mut self.grants {
            if grant.active && grant.is_expired_at(now) {
                grant.active = false;
                count += 1;
            }
        }
        count
    }

    /// Human-readable lines for the currently effective grants of one dApp.
    /// Each line names the capability *and* spells out what it discloses.
    pub fn list_for(&self, dapp: &DappId) -> Vec<String> {
        if !self.recognizes(dapp) {
            return Vec::new();
        }
        self.grants
            .iter()
            .filter(|grant| grant.dapp == *dapp && grant.active && !grant.is_expired_at(self.clock))
            .map(|grant| {
                let expiry = match grant.expires_at {
                    Some(expires) => format!("expires at {expires}"),
                    None => "no expiry".to_string(),
                };
                let mut line = format!(
                    "[{}] {} ({}) — scope: {}; issued at {}; {}; revoke with {}",
                    grant.id,
                    grant.capability,
                    grant.capability.human_label(),
                    grant.scope,
                    grant.issued_at,
                    expiry,
                    grant.id
                );
                for disclosure_line in grant.disclosure.to_human_lines() {
                    line.push('\n');
                    line.push_str(&disclosure_line);
                }
                line
            })
            .collect()
    }
}

/// A plain-language statement attached to every opaque proof handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofDisclosure {
    text: String,
}

impl ProofDisclosure {
    /// The canonical text shown to a user asked to approve a proof request.
    pub fn canonical() -> Self {
        Self {
            text: "This proof reveals only a yes/no answer to the statement that was requested. \
                   It does NOT reveal your account address, your balances, which NFTs you own, \
                   any specific token or collection, or any ability to move funds. \
                   No account access is required to produce it."
                .to_string(),
        }
    }

    /// The plain-language disclosure text.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Display for ProofDisclosure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

/// An opaque proof handle. It deliberately contains no account, balance, NFT,
/// or token data — only a serial number and a disclosure statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofHandle {
    serial: u64,
    disclosure: ProofDisclosure,
}

impl ProofHandle {
    /// Opaque serial number of this proof.
    pub fn serial(&self) -> u64 {
        self.serial
    }

    /// What this proof does and does not reveal.
    pub fn disclosure(&self) -> &ProofDisclosure {
        &self.disclosure
    }
}

/// A mock NFT reference returned by [`ConsentCoordinator::enumerate_nfts`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NftRef {
    collection: CollectionId,
    token_id: String,
}

impl NftRef {
    /// The approved collection this mock token belongs to.
    pub fn collection(&self) -> &CollectionId {
        &self.collection
    }

    /// The mock token identifier.
    pub fn token_id(&self) -> &str {
        &self.token_id
    }
}

/// A transfer the dApp has asked to propose. It is inert until the user
/// explicitly authorizes this exact request id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingTransfer {
    request_id: u64,
    dapp: DappId,
    from: AccountId,
    to: AccountId,
    amount: u64,
}

impl PendingTransfer {
    /// The id used for per-request authorization.
    pub fn request_id(&self) -> u64 {
        self.request_id
    }

    /// The dApp that proposed the transfer.
    pub fn dapp(&self) -> &DappId {
        &self.dapp
    }

    /// The account the transfer would come from.
    pub fn from(&self) -> &AccountId {
        &self.from
    }

    /// The account the transfer would go to.
    pub fn to(&self) -> &AccountId {
        &self.to
    }

    /// The requested amount (mock units).
    pub fn amount(&self) -> u64 {
        self.amount
    }
}

/// A mocked transfer proposal. No signature exists and nothing is submitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferProposal {
    request_id: u64,
    description: String,
}

impl TransferProposal {
    /// The request that was explicitly authorized.
    pub fn request_id(&self) -> u64 {
        self.request_id
    }

    /// Plain-language description of the mock proposal.
    pub fn description(&self) -> &str {
        &self.description
    }
}

/// Enforces consent rules on top of a [`PermissionStore`].
#[derive(Debug)]
pub struct ConsentCoordinator {
    store: PermissionStore,
    next_request: u64,
    next_proof: u64,
    pending: Vec<PendingTransfer>,
    authorized: Vec<u64>,
}

impl Default for ConsentCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsentCoordinator {
    /// Create a coordinator with a fresh, empty store.
    pub fn new() -> Self {
        Self {
            store: PermissionStore::new(),
            next_request: 1,
            next_proof: 1,
            pending: Vec::new(),
            authorized: Vec::new(),
        }
    }

    /// Register a dApp with the underlying store.
    pub fn register_dapp(&mut self, label: impl Into<String>) -> DappId {
        self.store.register_dapp(label)
    }

    /// Read-only access to the permission store (for listing and auditing).
    pub fn store(&self) -> &PermissionStore {
        &self.store
    }

    /// Mutable access to the permission store (for revocation and expiry).
    pub fn store_mut(&mut self) -> &mut PermissionStore {
        &mut self.store
    }

    /// Set the logical clock used for expiry checks.
    pub fn set_now(&mut self, now: u64) {
        self.store.set_now(now);
    }

    /// Approve a proof request **only**. Grants no account access.
    pub fn approve_proof_request(
        &mut self,
        dapp: &DappId,
        issued_at: u64,
        expires_at: Option<u64>,
    ) -> Result<GrantId, Error> {
        self.store.grant(
            dapp,
            Capability::ProofRequest,
            &Scope::Unspecified,
            issued_at,
            expires_at,
        )
    }

    /// Approve account access **only**, for one account. Grants no proof right.
    pub fn approve_account_access(
        &mut self,
        dapp: &DappId,
        account: &AccountId,
        issued_at: u64,
        expires_at: Option<u64>,
    ) -> Result<GrantId, Error> {
        self.store.grant(
            dapp,
            Capability::AccountAccess,
            &Scope::Account(account.clone()),
            issued_at,
            expires_at,
        )
    }

    /// Approve one other capability in one scope. Still single-capability.
    pub fn approve(
        &mut self,
        dapp: &DappId,
        capability: Capability,
        scope: &Scope,
        issued_at: u64,
        expires_at: Option<u64>,
    ) -> Result<GrantId, Error> {
        self.store
            .grant(dapp, capability, scope, issued_at, expires_at)
    }

    /// Bundled approval is deliberately unavailable. Each capability must be
    /// approved by its own call so the user can see and revoke it separately.
    pub fn approve_bundle(
        &mut self,
        _dapp: &DappId,
        _capabilities: &[Capability],
        _issued_at: u64,
        _expires_at: Option<u64>,
    ) -> Result<Vec<GrantId>, Error> {
        Err(Error::BundledApprovalNotAllowed)
    }

    /// Check a capability for a dApp, distinguishing denial from scope error.
    fn require(&self, dapp: &DappId, capability: Capability, scope: &Scope) -> Result<(), Error> {
        if !self.store.recognizes(dapp) {
            return Err(Error::UnrecognizedDapp);
        }
        let now = self.store.now();
        if self.store.has_at(dapp, capability, scope, now) {
            return Ok(());
        }
        if self.store.has_any_scope_at(dapp, capability, now) {
            Err(Error::ScopeMismatch)
        } else {
            Err(Error::Denied)
        }
    }

    /// Produce an opaque proof handle. Requires ONLY a ProofRequest grant;
    /// account access is neither required nor implied.
    pub fn request_proof(&mut self, dapp: &DappId) -> Result<ProofHandle, Error> {
        self.require(dapp, Capability::ProofRequest, &Scope::Unspecified)?;
        let serial = self.next_proof;
        self.next_proof += 1;
        Ok(ProofHandle {
            serial,
            disclosure: ProofDisclosure::canonical(),
        })
    }

    /// Enumerate mock NFTs. Requires NftEnumeration scoped to this account.
    pub fn enumerate_nfts(&self, dapp: &DappId, account: &AccountId) -> Result<Vec<NftRef>, Error> {
        self.require(
            dapp,
            Capability::NftEnumeration,
            &Scope::Account(account.clone()),
        )?;
        Ok(vec![NftRef {
            collection: CollectionId::new("mock-collection"),
            token_id: "mock-token-1".to_string(),
        }])
    }

    /// Read a mock balance. Requires BalanceRead scoped to this account.
    pub fn read_balance(&self, dapp: &DappId, account: &AccountId) -> Result<u64, Error> {
        self.require(
            dapp,
            Capability::BalanceRead,
            &Scope::Account(account.clone()),
        )?;
        Ok(0)
    }

    /// Ask the user to propose a transfer. Creates an inert pending request;
    /// it does not sign, submit, or move anything.
    pub fn propose_transfer(
        &mut self,
        dapp: &DappId,
        from: &AccountId,
        to: &AccountId,
        amount: u64,
    ) -> Result<PendingTransfer, Error> {
        self.require(
            dapp,
            Capability::ProposeTransfer,
            &Scope::Account(from.clone()),
        )?;
        let request_id = self.next_request;
        self.next_request += 1;
        let pending = PendingTransfer {
            request_id,
            dapp: *dapp,
            from: from.clone(),
            to: to.clone(),
            amount,
        };
        self.pending.push(pending.clone());
        Ok(pending)
    }

    /// Explicit, per-request authorization. Without this call no proposal is
    /// produced, and nothing can be signed or submitted.
    pub fn authorize_transfer(
        &mut self,
        dapp: &DappId,
        request_id: u64,
    ) -> Result<TransferProposal, Error> {
        if !self.store.recognizes(dapp) {
            return Err(Error::UnrecognizedDapp);
        }
        let pending = self
            .pending
            .iter()
            .find(|request| request.request_id == request_id)
            .cloned()
            .ok_or(Error::UnknownRequest)?;
        if pending.dapp != *dapp {
            return Err(Error::UnknownRequest);
        }
        if self.authorized.contains(&request_id) {
            return Err(Error::AlreadyAuthorized);
        }
        self.require(
            dapp,
            Capability::ProposeTransfer,
            &Scope::Account(pending.from.clone()),
        )?;
        self.authorized.push(request_id);
        Ok(TransferProposal {
            request_id,
            description: format!(
                "Mock proposal: transfer {} from {} to {} (not signed, not submitted)",
                pending.amount, pending.from, pending.to
            ),
        })
    }

    /// Whether a specific pending request has been explicitly authorized.
    pub fn is_authorized(&self, request_id: u64) -> bool {
        self.authorized.contains(&request_id)
    }

    /// Number of pending (not yet authorized) requests.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(label: &str) -> AccountId {
        AccountId::new(label)
    }

    fn dapp() -> (PermissionStore, DappId) {
        let mut store = PermissionStore::new();
        let id = store.register_dapp("test dApp");
        (store, id)
    }

    fn coordinator() -> (ConsentCoordinator, DappId) {
        let mut coordinator = ConsentCoordinator::new();
        let id = coordinator.register_dapp("test dApp");
        (coordinator, id)
    }

    #[test]
    fn deny_all_by_default() {
        let (store, id) = dapp();
        let account = account("acct-1");
        let scope = Scope::Account(account);
        assert!(!store.has(&id, Capability::ProofRequest, &Scope::Unspecified));
        assert!(!store.has(&id, Capability::AccountAccess, &scope));
        assert!(!store.has(&id, Capability::NftEnumeration, &scope));
        assert!(!store.has(&id, Capability::ProposeTransfer, &scope));
        assert!(!store.has(&id, Capability::BalanceRead, &scope));
        assert!(!store.has(&id, Capability::SignTransaction, &scope));
    }

    #[test]
    fn proof_only_can_request_proof() {
        let (mut coordinator, id) = coordinator();
        coordinator.approve_proof_request(&id, 0, None).unwrap();
        let handle = coordinator.request_proof(&id).unwrap();
        assert_eq!(handle.serial(), 1);
        assert!(handle.disclosure().text().contains("does NOT reveal"));
    }

    #[test]
    fn proof_only_cannot_enumerate_nfts() {
        let (mut coordinator, id) = coordinator();
        coordinator.approve_proof_request(&id, 0, None).unwrap();
        let result = coordinator.enumerate_nfts(&id, &account("acct-secret-1"));
        assert_eq!(result.unwrap_err(), Error::Denied);
    }

    #[test]
    fn proof_only_cannot_propose_transfer() {
        let (mut coordinator, id) = coordinator();
        coordinator.approve_proof_request(&id, 0, None).unwrap();
        let result = coordinator.propose_transfer(
            &id,
            &account("acct-secret-2"),
            &account("acct-other"),
            42,
        );
        assert_eq!(result.unwrap_err(), Error::Denied);
    }

    #[test]
    fn proof_only_cannot_read_balances() {
        let (mut coordinator, id) = coordinator();
        coordinator.approve_proof_request(&id, 0, None).unwrap();
        assert!(!coordinator.store().has(
            &id,
            Capability::BalanceRead,
            &Scope::Account(account("acct-secret-3"))
        ));
        assert_eq!(
            coordinator
                .read_balance(&id, &account("acct-secret-3"))
                .unwrap_err(),
            Error::Denied
        );
    }

    #[test]
    fn granting_proof_does_not_grant_account_access() {
        let (mut coordinator, id) = coordinator();
        coordinator.approve_proof_request(&id, 0, None).unwrap();
        assert!(coordinator
            .store()
            .has(&id, Capability::ProofRequest, &Scope::Unspecified));
        assert!(!coordinator.store().has(
            &id,
            Capability::AccountAccess,
            &Scope::Account(account("acct-1"))
        ));
    }

    #[test]
    fn proof_and_account_approval_are_separate_and_not_bundled() {
        let (mut coordinator, id) = coordinator();
        let caps = [Capability::ProofRequest, Capability::AccountAccess];
        assert_eq!(
            coordinator.approve_bundle(&id, &caps, 0, None).unwrap_err(),
            Error::BundledApprovalNotAllowed
        );
        assert!(coordinator.store().list_for(&id).is_empty());
        let proof = coordinator.approve_proof_request(&id, 0, None).unwrap();
        let access = coordinator
            .approve_account_access(&id, &account("acct-1"), 0, None)
            .unwrap();
        assert_ne!(proof, access);
        assert_eq!(coordinator.store().list_for(&id).len(), 2);
        assert!(coordinator.request_proof(&id).is_ok());
        assert!(coordinator.store().has(
            &id,
            Capability::AccountAccess,
            &Scope::Account(account("acct-1"))
        ));
    }

    #[test]
    fn revoking_one_capability_leaves_others() {
        let (mut store, id) = dapp();
        let scope = Scope::Account(account("acct-1"));
        let proof = store
            .grant(&id, Capability::ProofRequest, &Scope::Unspecified, 0, None)
            .unwrap();
        let access = store
            .grant(&id, Capability::AccountAccess, &scope, 0, None)
            .unwrap();
        store.revoke(proof).unwrap();
        assert!(!store.has(&id, Capability::ProofRequest, &Scope::Unspecified));
        assert!(store.has(&id, Capability::AccountAccess, &scope));
        assert!(!store.get(proof).unwrap().is_active());
        assert!(store.get(access).unwrap().is_active());
    }

    #[test]
    fn revoking_account_access_does_not_affect_proof() {
        let (mut coordinator, id) = coordinator();
        coordinator.approve_proof_request(&id, 0, None).unwrap();
        let access = coordinator
            .approve_account_access(&id, &account("acct-1"), 0, None)
            .unwrap();
        coordinator.store_mut().revoke(access).unwrap();
        assert!(coordinator.request_proof(&id).is_ok());
        assert!(!coordinator.store().has(
            &id,
            Capability::AccountAccess,
            &Scope::Account(account("acct-1"))
        ));
    }

    #[test]
    fn revoke_all_only_affects_one_dapp() {
        let (mut store, first) = dapp();
        let second = store.register_dapp("second dApp");
        store
            .grant(
                &first,
                Capability::ProofRequest,
                &Scope::Unspecified,
                0,
                None,
            )
            .unwrap();
        store
            .grant(
                &second,
                Capability::ProofRequest,
                &Scope::Unspecified,
                0,
                None,
            )
            .unwrap();
        assert_eq!(store.revoke_all(&first).unwrap(), 1);
        assert!(!store.has(&first, Capability::ProofRequest, &Scope::Unspecified));
        assert!(store.has(&second, Capability::ProofRequest, &Scope::Unspecified));
    }

    #[test]
    fn replay_of_revoked_grant_is_error() {
        let (mut store, id) = dapp();
        let grant_id = store
            .grant(&id, Capability::ProofRequest, &Scope::Unspecified, 0, None)
            .unwrap();
        store.revoke(grant_id).unwrap();
        assert_eq!(store.revoke(grant_id).unwrap_err(), Error::AlreadyRevoked);
        assert!(!store.has(&id, Capability::ProofRequest, &Scope::Unspecified));
        assert_eq!(
            store.revoke(GrantId(9999)).unwrap_err(),
            Error::GrantNotFound
        );
    }

    #[test]
    fn expired_grants_denied_and_revocable() {
        let (mut store, id) = dapp();
        store
            .grant(
                &id,
                Capability::ProofRequest,
                &Scope::Unspecified,
                10,
                Some(20),
            )
            .unwrap();
        store.set_now(20);
        assert!(store.has(&id, Capability::ProofRequest, &Scope::Unspecified));
        store.set_now(21);
        assert!(!store.has(&id, Capability::ProofRequest, &Scope::Unspecified));
        assert_eq!(store.revoke_expired(21), 1);
        assert_eq!(store.revoke_expired(21), 0);
    }

    #[test]
    fn revoke_expired_leaves_unexpired_grants() {
        let (mut store, id) = dapp();
        store
            .grant(
                &id,
                Capability::ProofRequest,
                &Scope::Unspecified,
                0,
                Some(5),
            )
            .unwrap();
        store
            .grant(
                &id,
                Capability::BalanceRead,
                &Scope::Account(account("acct-1")),
                0,
                Some(50),
            )
            .unwrap();
        assert_eq!(store.revoke_expired(10), 1);
        assert!(!store.has(&id, Capability::ProofRequest, &Scope::Unspecified));
        assert!(store.has(
            &id,
            Capability::BalanceRead,
            &Scope::Account(account("acct-1"))
        ));
    }

    #[test]
    fn forged_dapp_id_is_denied() {
        let (mut store, _real) = dapp();
        let forged = DappId {
            serial: 1,
            issuer: 0,
        };
        assert!(!store.recognizes(&forged));
        assert!(!store.has(&forged, Capability::ProofRequest, &Scope::Unspecified));
        assert!(store.list_for(&forged).is_empty());
        assert_eq!(
            store.revoke_all(&forged).unwrap_err(),
            Error::UnrecognizedDapp
        );
    }

    #[test]
    fn dapp_id_from_another_store_is_denied() {
        let (mut first, id_in_first) = dapp();
        let mut other = PermissionStore::new();
        first
            .grant(
                &id_in_first,
                Capability::ProofRequest,
                &Scope::Unspecified,
                0,
                None,
            )
            .unwrap();
        assert!(!other.recognizes(&id_in_first));
        assert!(!other.has(&id_in_first, Capability::ProofRequest, &Scope::Unspecified));
        assert_eq!(
            other.revoke_all(&id_in_first).unwrap_err(),
            Error::UnrecognizedDapp
        );
    }

    #[test]
    fn scope_mismatch_is_reported() {
        let (mut coordinator, id) = coordinator();
        coordinator
            .approve(
                &id,
                Capability::NftEnumeration,
                &Scope::Account(account("acct-a")),
                0,
                None,
            )
            .unwrap();
        assert_eq!(
            coordinator
                .enumerate_nfts(&id, &account("acct-b"))
                .unwrap_err(),
            Error::ScopeMismatch
        );
    }

    #[test]
    fn nft_enumeration_requires_account_scope() {
        let (mut coordinator, id) = coordinator();
        coordinator
            .approve(
                &id,
                Capability::NftEnumeration,
                &Scope::Unspecified,
                0,
                None,
            )
            .unwrap();
        assert_eq!(
            coordinator
                .enumerate_nfts(&id, &account("acct-1"))
                .unwrap_err(),
            Error::ScopeMismatch
        );
    }

    #[test]
    fn propose_transfer_requires_explicit_per_request_authorize() {
        let (mut coordinator, id) = coordinator();
        let from = account("acct-from");
        let to = account("acct-to");
        coordinator
            .approve(
                &id,
                Capability::ProposeTransfer,
                &Scope::Account(from.clone()),
                0,
                None,
            )
            .unwrap();
        let pending = coordinator.propose_transfer(&id, &from, &to, 7).unwrap();
        assert_eq!(coordinator.pending_count(), 1);
        assert!(!coordinator.is_authorized(pending.request_id()));
        assert_eq!(
            coordinator
                .authorize_transfer(&id, pending.request_id() + 99)
                .unwrap_err(),
            Error::UnknownRequest
        );
        let proposal = coordinator
            .authorize_transfer(&id, pending.request_id())
            .unwrap();
        assert_eq!(proposal.request_id(), pending.request_id());
        assert!(coordinator.is_authorized(pending.request_id()));
        assert!(proposal.description().contains("not signed, not submitted"));
        assert_eq!(
            coordinator
                .authorize_transfer(&id, pending.request_id())
                .unwrap_err(),
            Error::AlreadyAuthorized
        );
    }

    #[test]
    fn authorization_rechecks_the_grant() {
        let (mut coordinator, id) = coordinator();
        let from = account("acct-from-2");
        let grant_id = coordinator
            .approve(
                &id,
                Capability::ProposeTransfer,
                &Scope::Account(from.clone()),
                0,
                None,
            )
            .unwrap();
        let pending = coordinator
            .propose_transfer(&id, &from, &account("acct-to-2"), 1)
            .unwrap();
        coordinator.store_mut().revoke(grant_id).unwrap();
        assert_eq!(
            coordinator
                .authorize_transfer(&id, pending.request_id())
                .unwrap_err(),
            Error::Denied
        );
        assert!(!coordinator.is_authorized(pending.request_id()));
    }

    #[test]
    fn proof_handle_contains_no_account_or_token_data() {
        let (mut coordinator, id) = coordinator();
        coordinator.approve_proof_request(&id, 0, None).unwrap();
        // Distinctive identifiers that must never appear in a proof handle.
        let secret_account = account("acct-SUPER-SECRET-0xDEADBEEF");
        let secret_collection = CollectionId::new("mock-collection-TOP-SECRET");
        assert!(format!("{secret_account:?}{secret_collection:?}").contains("SUPER-SECRET"));

        let handle = coordinator.request_proof(&id).unwrap();
        let debug = format!("{handle:?}");
        assert!(!debug.contains("acct-"));
        assert!(!debug.contains("0x"));
        assert!(!debug.contains("SUPER-SECRET"));
        assert!(!debug.contains("TOP-SECRET"));
        assert!(!debug.contains("mock-token"));
        // The only extra content is the disclosure statement itself.
        assert!(debug.contains("ProofDisclosure"));
        assert!(debug.contains("does NOT reveal your account address"));
    }

    #[test]
    fn proof_disclosure_is_plain_language() {
        let disclosure = ProofDisclosure::canonical();
        let text = disclosure.text();
        assert!(text.contains("does NOT reveal your account address"));
        assert!(text.contains("balances"));
        assert!(text.contains("NFTs"));
        assert!(text.contains("token"));
        assert!(text.contains("No account access is required"));
    }

    #[test]
    fn listing_shows_disclosure_not_just_api_names() {
        let (mut store, id) = dapp();
        store
            .grant(&id, Capability::ProofRequest, &Scope::Unspecified, 1, None)
            .unwrap();
        store
            .grant(
                &id,
                Capability::BalanceRead,
                &Scope::Account(account("acct-1")),
                2,
                None,
            )
            .unwrap();
        let lines = store.list_for(&id).join("\n");
        assert!(lines.contains("ProofRequest"));
        assert!(lines.contains("Discloses:"));
        assert!(lines.contains("Does not disclose:"));
        assert!(lines.contains("your balances"));
        assert!(lines.contains("your account address"));
        assert!(lines.contains("only whether the statement you asked about is true or false"));
        assert!(lines.contains("revoke with grant#"));
    }

    #[test]
    fn list_for_hides_revoked_and_expired_grants() {
        let (mut store, id) = dapp();
        let proof = store
            .grant(&id, Capability::ProofRequest, &Scope::Unspecified, 0, None)
            .unwrap();
        let balance = store
            .grant(
                &id,
                Capability::BalanceRead,
                &Scope::Account(account("acct-1")),
                0,
                Some(5),
            )
            .unwrap();
        store.set_now(10);
        store.revoke(proof).unwrap();
        let lines = store.list_for(&id).join("\n");
        assert!(!lines.contains("ProofRequest"));
        assert!(!lines.contains("BalanceRead"));
        assert!(lines.is_empty());
        assert!(store.get(balance).unwrap().is_expired_at(10));
    }
}
