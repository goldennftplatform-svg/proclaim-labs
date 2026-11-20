// SPDX-License-Identifier: MIT AND Apache-2.0
//! Deterministic, dependency-free in-memory ledger simulator for the LP-0021
//! local vertical slice.
//!
//! # HONESTY / LIMITATIONS
//!
//! * This is **not** a LEZ chain, not a sequencer, and not testnet 0.4. Nothing
//!   here is submitted anywhere; all state lives in the current process.
//! * "Private" accounts are simulated with **opaque commitments and handles
//!   only**. [`PrivateCommitment`] and [`ReceiveHandle`] are labelled digests
//!   produced by a non-cryptographic FNV-style hash. This is **not real
//!   cryptography, not a zero-knowledge proof, and not a real shielded pool**.
//!   Balances are still stored in plain in the simulator so that the
//!   public/private balance invariants can be tested locally.
//! * [`transaction_hash`] is a deterministic non-cryptographic digest used only
//!   for reproducible event identity and replay checks. It is not a signature
//!   and provides no authenticity guarantee.
//!
//! The simulator exists solely to exercise deterministic ledger semantics:
//! balances, nonces, history scoping, event-driven restart reconstruction and
//! the differing event *shapes* of public versus private sends.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const MIX: u64 = 0x9e37_79b9_7f4a_7c15;
const HEX: &[u8; 16] = b"0123456789abcdef";

fn fnv64(bytes: &[u8], seed: u64) -> u64 {
    let mut hash = seed;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

/// Opaque account identifier. The inner serial is private; callers only ever
/// see the `acct-N` display form and cannot derive an address or key from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountId(u64);

impl AccountId {
    /// Construct an id from a serial. Generated ids come from [`Ledger`]; this
    /// constructor exists for deterministic fixtures and parsing only.
    pub const fn new(serial: u64) -> Self {
        Self(serial)
    }

    /// Process-local serial form, useful in tests and logs.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Display for AccountId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "acct-{}", self.0)
    }
}

/// Error returned when parsing an [`AccountId`] display form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseAccountIdError;

impl Display for ParseAccountIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "expected an opaque account id of the form acct-N")
    }
}

impl std::error::Error for ParseAccountIdError {}

impl FromStr for AccountId {
    type Err = ParseAccountIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let digits = value.strip_prefix("acct-").ok_or(ParseAccountIdError)?;
        if digits.is_empty() {
            return Err(ParseAccountIdError);
        }
        let serial = digits.parse::<u64>().map_err(|_| ParseAccountIdError)?;
        Ok(Self(serial))
    }
}

/// Whether an account is simulated as public or private.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccountKind {
    /// Transparent simulated account.
    Public,
    /// Simulated shielded account (handles only; no real cryptography).
    Private,
}

impl AccountKind {
    /// Short stable label used in events and output.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }
}

impl Display for AccountKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Errors when parsing an [`AccountKind`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseAccountKindError;

impl Display for ParseAccountKindError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "expected 'public' or 'private'")
    }
}

impl std::error::Error for ParseAccountKindError {}

impl FromStr for AccountKind {
    type Err = ParseAccountKindError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "public" => Ok(Self::Public),
            "private" => Ok(Self::Private),
            _ => Err(ParseAccountKindError),
        }
    }
}

/// Asset identifier. `Native` is the simulator's native asset; `MockToken` is
/// included only so balances are genuinely per-asset. No token program exists.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Asset {
    /// The simulated native asset.
    Native,
    /// A mock non-native asset id. This is not a real token program.
    MockToken(u32),
}

impl Asset {
    /// The native asset value.
    pub const fn native() -> Self {
        Self::Native
    }

    /// Stable textual id used in hashing and output.
    pub fn id(&self) -> String {
        match self {
            Self::Native => "native".to_owned(),
            Self::MockToken(id) => format!("mock-token-{id}"),
        }
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

/// Errors when parsing an [`Asset`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseAssetError;

impl Display for ParseAssetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "expected 'native' or 'mock-token-N'")
    }
}

impl std::error::Error for ParseAssetError {}

impl FromStr for Asset {
    type Err = ParseAssetError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "native" {
            return Ok(Self::Native);
        }
        let digits = value.strip_prefix("mock-token-").ok_or(ParseAssetError)?;
        let id = digits.parse::<u32>().map_err(|_| ParseAssetError)?;
        Ok(Self::MockToken(id))
    }
}

/// Handle for the native asset. Its methods aggregate *simulated* native
/// balances across accounts; no chain query is performed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeAsset;

impl NativeAsset {
    /// Construct the native asset handle.
    pub const fn new() -> Self {
        Self
    }

    /// The underlying [`Asset`] value.
    pub const fn asset(self) -> Asset {
        Asset::Native
    }

    /// Native balance of one account.
    pub fn balance(self, ledger: &Ledger, account: AccountId) -> Result<u128, LedgerError> {
        ledger.balance(account, Asset::Native)
    }

    /// Sum native balances across the supplied accounts. An unknown account is
    /// an error rather than a silently skipped entry.
    pub fn aggregate(self, ledger: &Ledger, accounts: &[AccountId]) -> Result<u128, LedgerError> {
        let mut total: u128 = 0;
        for account in accounts {
            total = total
                .checked_add(ledger.balance(*account, Asset::Native)?)
                .ok_or(LedgerError::Overflow)?;
        }
        Ok(total)
    }
}

/// A native-asset transfer proposal/transaction for the simulator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub from: AccountId,
    pub to: AccountId,
    pub amount: u128,
    pub asset: Asset,
    pub nonce: u64,
}

/// Deterministic, non-cryptographic digest of a [`Transaction`].
///
/// Four FNV-1a passes with different seeds are used to fill 32 bytes. This is
/// reproducible across processes and platforms, but it is **not** a signature
/// and must never be mistaken for one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TxHash([u8; 32]);

impl TxHash {
    /// Raw digest bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hex representation.
    pub fn as_hex(&self) -> String {
        bytes_to_hex(&self.0)
    }
}

impl Display for TxHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_hex())
    }
}

fn canonical_transaction_bytes(tx: &Transaction) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(b"lp0021-tx-v1");
    bytes.extend_from_slice(&tx.from.as_u64().to_le_bytes());
    bytes.extend_from_slice(&tx.to.as_u64().to_le_bytes());
    bytes.extend_from_slice(&tx.amount.to_le_bytes());
    let asset_id = tx.asset.id();
    bytes.extend_from_slice(&(asset_id.len() as u64).to_le_bytes());
    bytes.extend_from_slice(asset_id.as_bytes());
    bytes.extend_from_slice(&tx.nonce.to_le_bytes());
    bytes
}

/// Compute the deterministic simulator digest of `tx`.
pub fn transaction_hash(tx: &Transaction) -> TxHash {
    let canonical = canonical_transaction_bytes(tx);
    let mut digest = [0_u8; 32];
    for (index, chunk) in digest.chunks_mut(8).enumerate() {
        let seed = FNV_OFFSET.wrapping_add((index as u64).wrapping_mul(MIX));
        chunk.copy_from_slice(&fnv64(&canonical, seed).to_le_bytes());
    }
    TxHash(digest)
}

/// A simulated private-send handle.
///
/// HONESTY: this is a labelled digest only. It is **not** a cryptographic
/// commitment, hides nothing, and does not prove anything about a shielded
/// pool. It exists so private sends produce a different, recognisable event
/// shape while keeping the same balance invariants as public sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrivateCommitment {
    serial: u64,
    digest: [u8; 8],
}

impl PrivateCommitment {
    /// Construct a simulated handle from a serial and payload. Used to build
    /// deliberately invalid event-log fixtures in tests.
    pub const fn new(serial: u64, digest: [u8; 8]) -> Self {
        Self { serial, digest }
    }

    /// Derive the simulated handle for one transaction and digest.
    pub fn for_tx(tx: &Transaction, hash: &TxHash) -> Self {
        let serial = u64::from_le_bytes(
            hash.as_bytes()[0..8]
                .try_into()
                .expect("a 32-byte digest always has 8 leading bytes"),
        );
        let digest = fnv64(&canonical_transaction_bytes(tx), FNV_OFFSET ^ MIX).to_le_bytes();
        Self { serial, digest }
    }

    /// Opaque serial of the simulated handle.
    pub const fn serial(&self) -> u64 {
        self.serial
    }

    /// Hex form of the simulated handle payload.
    pub fn as_hex(&self) -> String {
        bytes_to_hex(&self.digest)
    }
}

impl Display for PrivateCommitment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "commit-sim-{:016x}-{}", self.serial, self.as_hex())
    }
}

/// An opaque receive handle generated by `receive`.
///
/// It contains no account id, address, key or asset amount. It is a local
/// serial plus a deterministic non-cryptographic digest, not a real payment
/// address or a shielded receiving key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReceiveHandle {
    serial: u64,
    kind: AccountKind,
    digest: [u8; 8],
}

impl ReceiveHandle {
    /// Create a simulated opaque receive handle.
    pub fn new(serial: u64, kind: AccountKind) -> Self {
        let mut seed_bytes = Vec::with_capacity(24);
        seed_bytes.extend_from_slice(b"lp0021-recv-v1");
        seed_bytes.extend_from_slice(kind.label().as_bytes());
        seed_bytes.extend_from_slice(&serial.to_le_bytes());
        Self {
            serial,
            kind,
            digest: fnv64(&seed_bytes, FNV_OFFSET).to_le_bytes(),
        }
    }

    /// The kind of account the handle was generated for.
    pub const fn kind(&self) -> AccountKind {
        self.kind
    }

    /// Opaque serial.
    pub const fn serial(&self) -> u64 {
        self.serial
    }

    /// Hex form of the simulated handle payload.
    pub fn as_hex(&self) -> String {
        bytes_to_hex(&self.digest)
    }
}

impl Display for ReceiveHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "recv-{}-{:08x}-{}",
            self.kind.label(),
            self.serial,
            self.as_hex()
        )
    }
}

/// Why a ledger operation was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerError {
    /// The account does not exist in this simulator.
    UnknownAccount(AccountId),
    /// The sender cannot cover the requested amount.
    InsufficientBalance {
        account: AccountId,
        asset: Asset,
        available: u128,
        required: u128,
    },
    /// The nonce was already consumed by this sender.
    NonceAlreadyUsed { account: AccountId, nonce: u64 },
    /// The sender must use its next sequential nonce.
    NonceOutOfOrder {
        account: AccountId,
        expected: u64,
        found: u64,
    },
    /// A transfer to the same account is rejected as meaningless.
    SameAccount(AccountId),
    /// Zero-value transfers are rejected.
    ZeroAmount,
    /// A simulated balance counter overflowed.
    Overflow,
}

impl Display for LedgerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAccount(account) => write!(f, "unknown account: {account}"),
            Self::InsufficientBalance {
                account,
                asset,
                available,
                required,
            } => write!(
                f,
                "insufficient {asset} balance for {account}: have {available}, need {required}"
            ),
            Self::NonceAlreadyUsed { account, nonce } => {
                write!(f, "nonce {nonce} already used by {account}")
            }
            Self::NonceOutOfOrder {
                account,
                expected,
                found,
            } => write!(
                f,
                "nonce out of order for {account}: expected {expected}, found {found}"
            ),
            Self::SameAccount(account) => {
                write!(f, "cannot transfer to the same account {account}")
            }
            Self::ZeroAmount => write!(f, "transfer amount must be positive"),
            Self::Overflow => write!(f, "simulated balance counter overflow"),
        }
    }
}

impl std::error::Error for LedgerError {}

/// Why reconstructing a ledger from an event log failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayError {
    /// The log itself contains a ledger-invalid transition.
    Ledger(LedgerError),
    /// The same account was opened twice.
    DuplicateAccount(AccountId),
    /// The recorded digest does not match the transaction.
    HashMismatch { expected: TxHash, actual: TxHash },
    /// A recorded private commitment does not match the transaction.
    CommitmentMismatch,
    /// A transfer event was recorded with the wrong account kind shape.
    ShapeMismatch {
        account: AccountId,
        expected: AccountKind,
        found: AccountKind,
    },
}

impl Display for ReplayError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ledger(error) => write!(f, "invalid ledger event: {error}"),
            Self::DuplicateAccount(account) => write!(f, "account {account} opened twice"),
            Self::HashMismatch { expected, actual } => {
                write!(
                    f,
                    "event digest mismatch: recorded {expected}, computed {actual}"
                )
            }
            Self::CommitmentMismatch => write!(f, "simulated private commitment does not match tx"),
            Self::ShapeMismatch {
                account,
                expected,
                found,
            } => write!(
                f,
                "event shape mismatch for {account}: expected {expected}, found {found}"
            ),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Direction of a history entry relative to the queried account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    /// Value left this account.
    Sent,
    /// Value arrived at this account.
    Received,
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sent => write!(f, "Sent"),
            Self::Received => write!(f, "Received"),
        }
    }
}

/// Event shape of a transfer. Public and private sends deliberately differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventShape {
    /// Transparent simulated transfer.
    Public,
    /// Simulated private transfer with an opaque handle.
    Private,
}

impl Display for EventShape {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Public => write!(f, "Public"),
            Self::Private => write!(f, "Private"),
        }
    }
}

/// One account's view of a transfer. History is strictly per-account: an entry
/// never leaks into an unrelated account's history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    pub hash: TxHash,
    pub from: AccountId,
    pub to: AccountId,
    pub amount: u128,
    pub asset: Asset,
    pub nonce: u64,
    pub direction: Direction,
    pub shape: EventShape,
}

/// Events are the sole source of truth for restart reconstruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerEvent {
    /// An account was opened with a given simulated kind.
    AccountOpened {
        account: AccountId,
        kind: AccountKind,
    },
    /// Simulated external deposit/funding (no faucet or chain involved).
    ExternalCredit {
        account: AccountId,
        asset: Asset,
        amount: u128,
    },
    /// Public send: no commitment field at all.
    TransferPublic { tx: Transaction, hash: TxHash },
    /// Private send: an extra simulated handle field, never on public sends.
    TransferPrivate {
        tx: Transaction,
        hash: TxHash,
        commitment: PrivateCommitment,
    },
}

impl LedgerEvent {
    /// The event shape, if this is a transfer event.
    pub fn shape(&self) -> Option<EventShape> {
        match self {
            Self::TransferPublic { .. } => Some(EventShape::Public),
            Self::TransferPrivate { .. } => Some(EventShape::Private),
            Self::AccountOpened { .. } | Self::ExternalCredit { .. } => None,
        }
    }
}

/// Result of applying a transfer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TxReceipt {
    pub hash: TxHash,
    pub event_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AccountState {
    kind: AccountKind,
    balances: BTreeMap<Asset, u128>,
    next_nonce: u64,
}

/// Deterministic in-memory ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ledger {
    accounts: BTreeMap<AccountId, AccountState>,
    events: Vec<LedgerEvent>,
    history: BTreeMap<AccountId, Vec<HistoryEntry>>,
    next_account: u64,
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger {
    /// Empty ledger. The first generated account id is `acct-1`.
    pub fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
            events: Vec::new(),
            history: BTreeMap::new(),
            next_account: 1,
        }
    }

    /// Open a simulated account and optionally credit it with a mock external
    /// native deposit. Returns the newly issued opaque id.
    pub fn create_account(
        &mut self,
        kind: AccountKind,
        initial_native: u128,
    ) -> Result<AccountId, LedgerError> {
        let serial = self.next_account;
        self.next_account = self
            .next_account
            .checked_add(1)
            .ok_or(LedgerError::Overflow)?;
        let account = AccountId::new(serial);
        self.commit_event(LedgerEvent::AccountOpened { account, kind });
        if initial_native > 0 {
            self.credit(account, Asset::Native, initial_native)?;
        }
        Ok(account)
    }

    /// Simulate an external credit. This is a local mock deposit, not a faucet,
    /// bridge or chain transaction.
    pub fn credit(
        &mut self,
        account: AccountId,
        asset: Asset,
        amount: u128,
    ) -> Result<LedgerEvent, LedgerError> {
        if !self.accounts.contains_key(&account) {
            return Err(LedgerError::UnknownAccount(account));
        }
        if amount == 0 {
            return Err(LedgerError::ZeroAmount);
        }
        let event = LedgerEvent::ExternalCredit {
            account,
            asset,
            amount,
        };
        self.commit_event(event.clone());
        Ok(event)
    }

    /// Kind of an account.
    pub fn account_kind(&self, account: AccountId) -> Result<AccountKind, LedgerError> {
        self.accounts
            .get(&account)
            .map(|state| state.kind)
            .ok_or(LedgerError::UnknownAccount(account))
    }

    /// Next nonce the sender must use.
    pub fn next_nonce(&self, account: AccountId) -> Result<u64, LedgerError> {
        self.accounts
            .get(&account)
            .map(|state| state.next_nonce)
            .ok_or(LedgerError::UnknownAccount(account))
    }

    /// Balance of one account in one asset. Unknown accounts are errors so a
    /// caller can never mistake "no such account" for a zero balance.
    pub fn balance(&self, account: AccountId, asset: Asset) -> Result<u128, LedgerError> {
        let state = self
            .accounts
            .get(&account)
            .ok_or(LedgerError::UnknownAccount(account))?;
        Ok(state.balances.get(&asset).copied().unwrap_or(0))
    }

    /// Native balance of one account.
    pub fn native_balance(&self, account: AccountId) -> Result<u128, LedgerError> {
        self.balance(account, Asset::Native)
    }

    /// Per-account history. Entries are only inserted for `from` and `to`, so
    /// unrelated accounts cannot observe a transfer.
    pub fn history(&self, account: AccountId) -> Result<Vec<HistoryEntry>, LedgerError> {
        if !self.accounts.contains_key(&account) {
            return Err(LedgerError::UnknownAccount(account));
        }
        Ok(self.history.get(&account).cloned().unwrap_or_default())
    }

    /// Append-only event log used for restart reconstruction.
    pub fn events(&self) -> &[LedgerEvent] {
        &self.events
    }

    /// Validate then apply a transfer. Nothing is signed or sent anywhere.
    pub fn apply(&mut self, tx: Transaction) -> Result<TxReceipt, LedgerError> {
        self.check_transfer(&tx)?;
        let from_kind = self.accounts[&tx.from].kind;
        let hash = transaction_hash(&tx);
        let event = match from_kind {
            AccountKind::Public => LedgerEvent::TransferPublic { tx, hash },
            AccountKind::Private => LedgerEvent::TransferPrivate {
                commitment: PrivateCommitment::for_tx(&tx, &hash),
                tx,
                hash,
            },
        };
        // The sender's kind decides the event shape; both balances are updated.
        self.commit_event(event);
        Ok(TxReceipt {
            hash,
            event_index: self.events.len() - 1,
        })
    }

    /// Reconstruct a ledger from a previously recorded event log.
    pub fn replay(events: &[LedgerEvent]) -> Result<Self, ReplayError> {
        replay(events)
    }

    fn check_transfer(&self, tx: &Transaction) -> Result<(), LedgerError> {
        let state = self
            .accounts
            .get(&tx.from)
            .ok_or(LedgerError::UnknownAccount(tx.from))?;
        if !self.accounts.contains_key(&tx.to) {
            return Err(LedgerError::UnknownAccount(tx.to));
        }
        if tx.from == tx.to {
            return Err(LedgerError::SameAccount(tx.from));
        }
        if tx.amount == 0 {
            return Err(LedgerError::ZeroAmount);
        }
        if tx.nonce < state.next_nonce {
            return Err(LedgerError::NonceAlreadyUsed {
                account: tx.from,
                nonce: tx.nonce,
            });
        }
        if tx.nonce > state.next_nonce {
            return Err(LedgerError::NonceOutOfOrder {
                account: tx.from,
                expected: state.next_nonce,
                found: tx.nonce,
            });
        }
        let available = state.balances.get(&tx.asset).copied().unwrap_or(0);
        if available < tx.amount {
            return Err(LedgerError::InsufficientBalance {
                account: tx.from,
                asset: tx.asset.clone(),
                available,
                required: tx.amount,
            });
        }
        Ok(())
    }

    fn validate_event(&self, event: &LedgerEvent) -> Result<(), ReplayError> {
        match event {
            LedgerEvent::AccountOpened { account, .. } => {
                if self.accounts.contains_key(account) {
                    Err(ReplayError::DuplicateAccount(*account))
                } else {
                    Ok(())
                }
            }
            LedgerEvent::ExternalCredit {
                account,
                asset,
                amount,
            } => {
                if !self.accounts.contains_key(account) {
                    return Err(ReplayError::Ledger(LedgerError::UnknownAccount(*account)));
                }
                if *amount == 0 {
                    return Err(ReplayError::Ledger(LedgerError::ZeroAmount));
                }
                let current = self
                    .balance(*account, asset.clone())
                    .map_err(ReplayError::Ledger)?;
                current
                    .checked_add(*amount)
                    .ok_or(ReplayError::Ledger(LedgerError::Overflow))?;
                Ok(())
            }
            LedgerEvent::TransferPublic { tx, hash } => {
                self.validate_transfer_event(tx, hash, None, EventShape::Public)
            }
            LedgerEvent::TransferPrivate {
                tx,
                hash,
                commitment,
            } => self.validate_transfer_event(tx, hash, Some(*commitment), EventShape::Private),
        }
    }

    fn validate_transfer_event(
        &self,
        tx: &Transaction,
        hash: &TxHash,
        commitment: Option<PrivateCommitment>,
        shape: EventShape,
    ) -> Result<(), ReplayError> {
        self.check_transfer(tx).map_err(ReplayError::Ledger)?;
        let computed_hash = transaction_hash(tx);
        if *hash != computed_hash {
            return Err(ReplayError::HashMismatch {
                expected: *hash,
                actual: computed_hash,
            });
        }
        let actual_kind = self.accounts[&tx.from].kind;
        let expected_kind = match shape {
            EventShape::Public => AccountKind::Public,
            EventShape::Private => AccountKind::Private,
        };
        if actual_kind != expected_kind {
            return Err(ReplayError::ShapeMismatch {
                account: tx.from,
                expected: expected_kind,
                found: actual_kind,
            });
        }
        if shape == EventShape::Private {
            let expected = PrivateCommitment::for_tx(tx, hash);
            if commitment != Some(expected) {
                return Err(ReplayError::CommitmentMismatch);
            }
        }
        Ok(())
    }

    fn commit_event(&mut self, event: LedgerEvent) {
        match &event {
            LedgerEvent::AccountOpened { account, kind } => {
                self.accounts.insert(
                    *account,
                    AccountState {
                        kind: *kind,
                        balances: BTreeMap::new(),
                        next_nonce: 0,
                    },
                );
                self.history.insert(*account, Vec::new());
            }
            LedgerEvent::ExternalCredit {
                account,
                asset,
                amount,
            } => {
                let state = self
                    .accounts
                    .get_mut(account)
                    .expect("validated before commit");
                let balance = state.balances.entry(asset.clone()).or_insert(0);
                *balance = balance
                    .checked_add(*amount)
                    .expect("validated before commit");
            }
            LedgerEvent::TransferPublic { tx, hash } => {
                self.commit_transfer(tx, *hash, EventShape::Public);
            }
            LedgerEvent::TransferPrivate { tx, hash, .. } => {
                self.commit_transfer(tx, *hash, EventShape::Private);
            }
        }
        self.events.push(event);
    }

    fn commit_transfer(&mut self, tx: &Transaction, hash: TxHash, shape: EventShape) {
        {
            let from = self
                .accounts
                .get_mut(&tx.from)
                .expect("validated before commit");
            let balance = from.balances.entry(tx.asset.clone()).or_insert(0);
            *balance = balance
                .checked_sub(tx.amount)
                .expect("validated before commit");
            from.next_nonce = from
                .next_nonce
                .checked_add(1)
                .expect("validated before commit");
        }
        {
            let to = self
                .accounts
                .get_mut(&tx.to)
                .expect("validated before commit");
            let balance = to.balances.entry(tx.asset.clone()).or_insert(0);
            *balance = balance
                .checked_add(tx.amount)
                .expect("validated before commit");
        }
        let sent = HistoryEntry {
            hash,
            from: tx.from,
            to: tx.to,
            amount: tx.amount,
            asset: tx.asset.clone(),
            nonce: tx.nonce,
            direction: Direction::Sent,
            shape,
        };
        let received = HistoryEntry {
            direction: Direction::Received,
            ..sent.clone()
        };
        self.history
            .get_mut(&tx.from)
            .expect("validated before commit")
            .push(sent);
        self.history
            .get_mut(&tx.to)
            .expect("validated before commit")
            .push(received);
    }
}

/// Reconstruct a [`Ledger`] from an ordered event log.
pub fn replay(events: &[LedgerEvent]) -> Result<Ledger, ReplayError> {
    let mut ledger = Ledger::new();
    for event in events {
        ledger.validate_event(event)?;
        ledger.commit_event(event.clone());
    }
    let highest = events
        .iter()
        .filter_map(|event| match event {
            LedgerEvent::AccountOpened { account, .. } => Some(account.as_u64()),
            _ => None,
        })
        .max();
    ledger.next_account = highest.map_or(1, |serial| serial.saturating_add(1));
    Ok(ledger)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn funded_pair() -> (Ledger, AccountId, AccountId) {
        let mut ledger = Ledger::new();
        let alice = ledger.create_account(AccountKind::Public, 100).unwrap();
        let bob = ledger.create_account(AccountKind::Private, 50).unwrap();
        (ledger, alice, bob)
    }

    #[test]
    fn account_ids_are_opaque_and_kinds_distinct() {
        let (ledger, alice, bob) = funded_pair();
        assert_eq!(alice.as_u64(), 1);
        assert_eq!(bob.as_u64(), 2);
        assert_eq!(alice.to_string(), "acct-1");
        assert_eq!("acct-2".parse::<AccountId>(), Ok(bob));
        assert!("acct-".parse::<AccountId>().is_err());
        assert_eq!(ledger.account_kind(alice).unwrap(), AccountKind::Public);
        assert_eq!(ledger.account_kind(bob).unwrap(), AccountKind::Private);
    }

    #[test]
    fn native_asset_aggregates_public_and_private_balances() {
        let (ledger, alice, bob) = funded_pair();
        assert_eq!(NativeAsset::new().balance(&ledger, alice), Ok(100));
        assert_eq!(
            NativeAsset::new().aggregate(&ledger, &[alice, bob]),
            Ok(150)
        );
        assert_eq!(ledger.native_balance(bob), Ok(50));
        assert!(NativeAsset::new()
            .aggregate(&ledger, &[alice, AccountId::new(999)])
            .is_err());
    }

    #[test]
    fn public_transfer_moves_balances_and_records_history() {
        let (mut ledger, alice, bob) = funded_pair();
        let tx = Transaction {
            from: alice,
            to: bob,
            amount: 30,
            asset: Asset::Native,
            nonce: 0,
        };
        let receipt = ledger.apply(tx.clone()).unwrap();
        assert_eq!(receipt.hash, transaction_hash(&tx));
        assert_eq!(ledger.balance(alice, Asset::Native), Ok(70));
        assert_eq!(ledger.balance(bob, Asset::Native), Ok(80));
        assert_eq!(ledger.next_nonce(alice), Ok(1));
        let sent = ledger.history(alice).unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].direction, Direction::Sent);
        assert_eq!(sent[0].shape, EventShape::Public);
        let received = ledger.history(bob).unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].direction, Direction::Received);
    }

    #[test]
    fn private_transfer_uses_distinct_event_shape() {
        let (mut ledger, alice, bob) = funded_pair();
        let tx = Transaction {
            from: bob,
            to: alice,
            amount: 10,
            asset: Asset::Native,
            nonce: 0,
        };
        ledger.apply(tx.clone()).unwrap();
        let event = &ledger.events()[4];
        match event {
            LedgerEvent::TransferPrivate {
                tx: recorded,
                commitment,
                ..
            } => {
                assert_eq!(*recorded, tx);
                assert_eq!(
                    *commitment,
                    PrivateCommitment::for_tx(&tx, &transaction_hash(&tx))
                );
                assert!(commitment.to_string().starts_with("commit-sim-"));
            }
            other => panic!("expected simulated private event shape, got {other:?}"),
        }
        assert_eq!(ledger.history(bob).unwrap()[0].shape, EventShape::Private);
        assert_eq!(ledger.balance(bob, Asset::Native), Ok(40));
        assert_eq!(ledger.balance(alice, Asset::Native), Ok(110));
    }

    #[test]
    fn insufficient_balance_is_rejected() {
        let (mut ledger, alice, bob) = funded_pair();
        let tx = Transaction {
            from: alice,
            to: bob,
            amount: 101,
            asset: Asset::Native,
            nonce: 0,
        };
        assert_eq!(
            ledger.apply(tx),
            Err(LedgerError::InsufficientBalance {
                account: alice,
                asset: Asset::Native,
                available: 100,
                required: 101,
            })
        );
        assert_eq!(ledger.events().len(), 4);
        assert_eq!(ledger.balance(alice, Asset::Native), Ok(100));
    }

    #[test]
    fn nonce_replay_is_rejected() {
        let (mut ledger, alice, bob) = funded_pair();
        let tx = Transaction {
            from: alice,
            to: bob,
            amount: 1,
            asset: Asset::Native,
            nonce: 0,
        };
        ledger.apply(tx.clone()).unwrap();
        assert_eq!(
            ledger.apply(tx),
            Err(LedgerError::NonceAlreadyUsed {
                account: alice,
                nonce: 0
            })
        );
        assert_eq!(ledger.events().len(), 5);
    }

    #[test]
    fn out_of_order_nonce_is_rejected() {
        let (mut ledger, alice, bob) = funded_pair();
        let tx = Transaction {
            from: alice,
            to: bob,
            amount: 1,
            asset: Asset::Native,
            nonce: 4,
        };
        assert_eq!(
            ledger.apply(tx),
            Err(LedgerError::NonceOutOfOrder {
                account: alice,
                expected: 0,
                found: 4,
            })
        );
    }

    #[test]
    fn transaction_hash_is_deterministic_and_field_sensitive() {
        let (_, alice, bob) = funded_pair();
        let tx = Transaction {
            from: alice,
            to: bob,
            amount: 7,
            asset: Asset::Native,
            nonce: 2,
        };
        assert_eq!(transaction_hash(&tx), transaction_hash(&tx));
        assert_eq!(transaction_hash(&tx).as_hex().len(), 64);
        let changed_amount = Transaction {
            amount: 8,
            ..tx.clone()
        };
        let changed_nonce = Transaction {
            nonce: 3,
            ..tx.clone()
        };
        let changed_asset = Transaction {
            asset: Asset::MockToken(1),
            ..tx
        };
        assert_ne!(transaction_hash(&tx), transaction_hash(&changed_amount));
        assert_ne!(transaction_hash(&tx), transaction_hash(&changed_nonce));
        assert_ne!(transaction_hash(&tx), transaction_hash(&changed_asset));
    }

    #[test]
    fn history_is_scoped_and_does_not_leak_across_accounts() {
        let mut ledger = Ledger::new();
        let alice = ledger.create_account(AccountKind::Public, 100).unwrap();
        let bob = ledger.create_account(AccountKind::Private, 0).unwrap();
        let carol = ledger.create_account(AccountKind::Public, 0).unwrap();
        ledger
            .apply(Transaction {
                from: alice,
                to: bob,
                amount: 5,
                asset: Asset::Native,
                nonce: 0,
            })
            .unwrap();
        assert!(ledger.history(carol).unwrap().is_empty());
        assert_eq!(ledger.native_balance(carol), Ok(0));
        assert_eq!(ledger.history(bob).unwrap().len(), 1);
        assert_eq!(ledger.history(alice).unwrap().len(), 1);
    }

    #[test]
    fn replay_reconstructs_balances_history_and_nonces() {
        let (mut ledger, alice, bob) = funded_pair();
        ledger
            .apply(Transaction {
                from: alice,
                to: bob,
                amount: 11,
                asset: Asset::Native,
                nonce: 0,
            })
            .unwrap();
        ledger
            .apply(Transaction {
                from: bob,
                to: alice,
                amount: 4,
                asset: Asset::Native,
                nonce: 0,
            })
            .unwrap();
        let events = ledger.events().to_vec();
        let restored = replay(&events).unwrap();
        assert_eq!(restored, ledger);
        assert_eq!(restored.balance(alice, Asset::Native), Ok(93));
        assert_eq!(restored.balance(bob, Asset::Native), Ok(57));
        assert_eq!(restored.next_nonce(alice), Ok(1));
        assert_eq!(restored.next_nonce(bob), Ok(1));
        assert_eq!(restored.history(alice).unwrap().len(), 2);
        assert_eq!(restored.history(bob).unwrap().len(), 2);
    }

    #[test]
    fn replay_rejects_tampered_digest_and_commitment() {
        let (mut ledger, alice, bob) = funded_pair();
        ledger
            .apply(Transaction {
                from: alice,
                to: bob,
                amount: 1,
                asset: Asset::Native,
                nonce: 0,
            })
            .unwrap();
        ledger
            .apply(Transaction {
                from: bob,
                to: alice,
                amount: 1,
                asset: Asset::Native,
                nonce: 0,
            })
            .unwrap();
        let mut events = ledger.events().to_vec();
        match &mut events[4] {
            LedgerEvent::TransferPublic { hash, .. } => {
                *hash = TxHash([0xff; 32]);
            }
            other => panic!("unexpected event {other:?}"),
        }
        assert!(matches!(
            replay(&events),
            Err(ReplayError::HashMismatch { .. })
        ));
        let mut events = ledger.events().to_vec();
        match &mut events[5] {
            LedgerEvent::TransferPrivate { commitment, .. } => {
                *commitment = PrivateCommitment::new(1, [0; 8]);
            }
            other => panic!("unexpected event {other:?}"),
        }
        assert_eq!(replay(&events), Err(ReplayError::CommitmentMismatch));
    }

    #[test]
    fn replay_rejects_duplicate_account_open() {
        let (ledger, alice, _) = funded_pair();
        let mut events = ledger.events().to_vec();
        events.push(LedgerEvent::AccountOpened {
            account: alice,
            kind: AccountKind::Public,
        });
        assert_eq!(replay(&events), Err(ReplayError::DuplicateAccount(alice)));
    }

    #[test]
    fn zero_amount_and_self_transfer_are_rejected() {
        let (mut ledger, alice, bob) = funded_pair();
        let zero = Transaction {
            from: alice,
            to: bob,
            amount: 0,
            asset: Asset::Native,
            nonce: 0,
        };
        assert_eq!(ledger.apply(zero.clone()), Err(LedgerError::ZeroAmount));
        let self_transfer = Transaction {
            from: alice,
            to: alice,
            amount: 1,
            ..zero
        };
        assert_eq!(
            ledger.apply(self_transfer),
            Err(LedgerError::SameAccount(alice))
        );
    }

    #[test]
    fn external_credit_replays_and_is_per_asset() {
        let mut ledger = Ledger::new();
        let alice = ledger.create_account(AccountKind::Public, 0).unwrap();
        let token = Asset::MockToken(7);
        ledger.credit(alice, token.clone(), 9).unwrap();
        ledger.credit(alice, Asset::Native, 3).unwrap();
        assert_eq!(ledger.balance(alice, token.clone()), Ok(9));
        assert_eq!(ledger.balance(alice, Asset::Native), Ok(3));
        let restored = replay(ledger.events()).unwrap();
        assert_eq!(restored.balance(alice, token), Ok(9));
        assert_eq!(restored.balance(alice, Asset::Native), Ok(3));
    }

    #[test]
    fn receive_handles_are_opaque_and_kind_specific() {
        let public = ReceiveHandle::new(1, AccountKind::Public);
        let private = ReceiveHandle::new(1, AccountKind::Private);
        assert_ne!(public, private);
        assert_eq!(public.kind(), AccountKind::Public);
        assert_eq!(private.serial(), 1);
        let rendered = public.to_string();
        assert!(rendered.starts_with("recv-public-"));
        assert!(!rendered.contains("acct-"));
        assert!(!rendered.contains("native"));
    }
}
