// SPDX-License-Identifier: MIT AND Apache-2.0
//! Small local reimplementation of the consent-store surface needed by the
//! wallet runtime: single-capability grants, explicit scopes, deny by default.
//! This is not a path dependency on `lp0021-permissions`; it is a local copy of
//! the essential rules for this self-contained staged slice.

use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

use wallet_chain::AccountId;

static NEXT_ISSUER: AtomicU64 = AtomicU64::new(1);

fn fresh_issuer() -> u64 {
    NEXT_ISSUER.fetch_add(1, Ordering::Relaxed)
}

/// Opaque, store-scoped dApp identity. Callers cannot fabricate one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DappId {
    serial: u64,
    issuer: u64,
}

impl Display for DappId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "dapp-{}", self.serial)
    }
}

/// A single grantable capability. Capabilities are never bundled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// Build (but never sign) a transfer proposal.
    ProposeTransfer,
    /// Produce a mock signature for an explicitly approved proposal.
    SignTransaction,
    /// Read a simulated balance or history for one account.
    BalanceRead,
    /// Generate an opaque receive handle.
    Receive,
}

impl Capability {
    /// Plain-language label for a prompt or audit line.
    pub const fn label(self) -> &'static str {
        match self {
            Self::ProposeTransfer => "transaction proposal",
            Self::SignTransaction => "transaction signature",
            Self::BalanceRead => "balance read",
            Self::Receive => "receive handle",
        }
    }
}

impl Display for Capability {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// What a capability applies to.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    /// No particular account (used for receive handles).
    Unspecified,
    /// Exactly one account.
    Account(AccountId),
}

impl Display for Scope {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unspecified => write!(f, "unspecified"),
            Self::Account(account) => write!(f, "account {account}"),
        }
    }
}

/// Opaque handle for one individually revocable grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GrantId(u64);

impl GrantId {
    /// Construct a grant id from a serial (test fixtures and parsing).
    pub const fn from_serial(serial: u64) -> Self {
        Self(serial)
    }
}

impl Display for GrantId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "grant-{}", self.0)
    }
}

/// Grant id parse failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseGrantIdError;

impl Display for ParseGrantIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "expected a grant id of the form grant-N")
    }
}

impl std::error::Error for ParseGrantIdError {}

impl FromStr for GrantId {
    type Err = ParseGrantIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let digits = value.strip_prefix("grant-").unwrap_or(value);
        let serial = digits.parse::<u64>().map_err(|_| ParseGrantIdError)?;
        Ok(Self(serial))
    }
}

/// Consent-check failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsentError {
    /// The dApp id was not issued by this store.
    UnrecognizedDapp,
    /// No active grant covers the request. Deny by default.
    Denied,
    /// A grant exists for this capability but for a different scope.
    ScopeMismatch,
    /// No grant has this id.
    GrantNotFound,
    /// The grant was already revoked.
    AlreadyRevoked,
}

impl Display for ConsentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnrecognizedDapp => "dApp id is not recognized by this wallet",
            Self::Denied => "permission denied (no matching active grant)",
            Self::ScopeMismatch => "permission granted only for a different scope",
            Self::GrantNotFound => "no grant has that id",
            Self::AlreadyRevoked => "grant was already revoked",
        };
        write!(f, "{message}")
    }
}

impl std::error::Error for ConsentError {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Grant {
    id: GrantId,
    dapp: DappId,
    capability: Capability,
    scope: Scope,
    active: bool,
}

/// In-memory, deny-by-default set of individual grants.
#[derive(Debug)]
pub struct PermissionStore {
    issuer: u64,
    next_dapp: u64,
    next_grant: u64,
    grants: Vec<Grant>,
}

impl Default for PermissionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionStore {
    /// Empty store with no grants.
    pub fn new() -> Self {
        Self {
            issuer: fresh_issuer(),
            next_dapp: 1,
            next_grant: 1,
            grants: Vec::new(),
        }
    }

    /// Register a dApp and return its opaque, store-scoped id.
    pub fn register_dapp(&mut self, _label: impl Into<String>) -> DappId {
        let dapp = DappId {
            serial: self.next_dapp,
            issuer: self.issuer,
        };
        self.next_dapp += 1;
        dapp
    }

    /// Whether the id came from this store.
    pub fn recognizes(&self, dapp: &DappId) -> bool {
        dapp.issuer == self.issuer && dapp.serial >= 1 && dapp.serial < self.next_dapp
    }

    /// Issue exactly one capability grant.
    pub fn grant(
        &mut self,
        dapp: &DappId,
        capability: Capability,
        scope: &Scope,
    ) -> Result<GrantId, ConsentError> {
        if !self.recognizes(dapp) {
            return Err(ConsentError::UnrecognizedDapp);
        }
        let id = GrantId(self.next_grant);
        self.next_grant += 1;
        self.grants.push(Grant {
            id,
            dapp: *dapp,
            capability,
            scope: scope.clone(),
            active: true,
        });
        Ok(id)
    }

    /// Deny-by-default check.
    pub fn has(&self, dapp: &DappId, capability: Capability, scope: &Scope) -> bool {
        self.check(dapp, capability, scope).is_ok()
    }

    /// Deny-by-default check with a reason.
    pub fn check(
        &self,
        dapp: &DappId,
        capability: Capability,
        scope: &Scope,
    ) -> Result<(), ConsentError> {
        if !self.recognizes(dapp) {
            return Err(ConsentError::UnrecognizedDapp);
        }
        let matching = self.grants.iter().any(|grant| {
            grant.dapp == *dapp
                && grant.capability == capability
                && grant.scope == *scope
                && grant.active
        });
        if matching {
            return Ok(());
        }
        let other_scope = self
            .grants
            .iter()
            .any(|grant| grant.dapp == *dapp && grant.capability == capability && grant.active);
        if other_scope {
            Err(ConsentError::ScopeMismatch)
        } else {
            Err(ConsentError::Denied)
        }
    }

    /// Revoke exactly one grant, leaving all others untouched.
    pub fn revoke(&mut self, id: GrantId) -> Result<(), ConsentError> {
        match self.grants.iter_mut().find(|grant| grant.id == id) {
            Some(grant) if grant.active => {
                grant.active = false;
                Ok(())
            }
            Some(_) => Err(ConsentError::AlreadyRevoked),
            None => Err(ConsentError::GrantNotFound),
        }
    }

    /// Active grants for one dApp, as audit lines.
    pub fn list_for(&self, dapp: &DappId) -> Vec<String> {
        self.grants
            .iter()
            .filter(|grant| grant.dapp == *dapp && grant.active)
            .map(|grant| {
                format!(
                    "{}: {} in {} (revoked with {})",
                    grant.id, grant.capability, grant.scope, grant.id
                )
            })
            .collect()
    }
}

/// Adapter boundary used by the runtime so consent checks are not coupled to
/// the concrete store implementation.
pub trait ConsentGate {
    /// Whether the request is allowed. Must deny by default.
    fn has(&self, dapp: &DappId, capability: Capability, scope: &Scope) -> bool;

    /// Same check with a reason; overridable for better diagnostics.
    fn check(
        &self,
        dapp: &DappId,
        capability: Capability,
        scope: &Scope,
    ) -> Result<(), ConsentError> {
        if self.has(dapp, capability, scope) {
            Ok(())
        } else {
            Err(ConsentError::Denied)
        }
    }
}

/// Shared-store adapter. Cloning the handle keeps the same underlying store.
#[derive(Clone, Debug)]
pub struct StoreGate {
    store: Rc<RefCell<PermissionStore>>,
}

impl StoreGate {
    /// Wrap a shared permission store.
    pub fn new(store: Rc<RefCell<PermissionStore>>) -> Self {
        Self { store }
    }

    /// Issue a grant through the shared store.
    pub fn grant(
        &self,
        dapp: &DappId,
        capability: Capability,
        scope: &Scope,
    ) -> Result<GrantId, ConsentError> {
        self.store.borrow_mut().grant(dapp, capability, scope)
    }

    /// Revoke one grant through the shared store.
    pub fn revoke(&self, id: GrantId) -> Result<(), ConsentError> {
        self.store.borrow_mut().revoke(id)
    }
}

impl ConsentGate for StoreGate {
    fn has(&self, dapp: &DappId, capability: Capability, scope: &Scope) -> bool {
        self.store.borrow().has(dapp, capability, scope)
    }

    fn check(
        &self,
        dapp: &DappId,
        capability: Capability,
        scope: &Scope,
    ) -> Result<(), ConsentError> {
        self.store.borrow().check(dapp, capability, scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_denies_everything_by_default() {
        let mut store = PermissionStore::new();
        let dapp = store.register_dapp("test");
        let scope = Scope::Account(AccountId::new(1));
        for capability in [
            Capability::ProposeTransfer,
            Capability::SignTransaction,
            Capability::BalanceRead,
            Capability::Receive,
        ] {
            assert_eq!(
                store.check(&dapp, capability, &scope),
                Err(ConsentError::Denied)
            );
        }
    }

    #[test]
    fn grants_are_single_capability_and_individually_revocable() {
        let mut store = PermissionStore::new();
        let dapp = store.register_dapp("test");
        let account = AccountId::new(1);
        let propose = store
            .grant(&dapp, Capability::ProposeTransfer, &Scope::Account(account))
            .unwrap();
        let sign = store
            .grant(&dapp, Capability::SignTransaction, &Scope::Account(account))
            .unwrap();
        assert_ne!(propose, sign);
        store.revoke(propose).unwrap();
        assert!(!store.has(&dapp, Capability::ProposeTransfer, &Scope::Account(account)));
        assert!(store.has(&dapp, Capability::SignTransaction, &Scope::Account(account)));
        assert_eq!(store.revoke(propose), Err(ConsentError::AlreadyRevoked));
    }

    #[test]
    fn wrong_scope_reports_scope_mismatch() {
        let mut store = PermissionStore::new();
        let dapp = store.register_dapp("test");
        store
            .grant(
                &dapp,
                Capability::BalanceRead,
                &Scope::Account(AccountId::new(1)),
            )
            .unwrap();
        assert_eq!(
            store.check(
                &dapp,
                Capability::BalanceRead,
                &Scope::Account(AccountId::new(2))
            ),
            Err(ConsentError::ScopeMismatch)
        );
    }

    #[test]
    fn foreign_dapp_id_is_unrecognized() {
        let mut first = PermissionStore::new();
        let second = PermissionStore::new();
        let id = first.register_dapp("first");
        assert!(!second.recognizes(&id));
        assert_eq!(
            second.check(&id, Capability::Receive, &Scope::Unspecified),
            Err(ConsentError::UnrecognizedDapp)
        );
    }
}
