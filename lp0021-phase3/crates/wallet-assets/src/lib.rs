// SPDX-License-Identifier: MIT AND Apache-2.0
//! Dependency-free, in-memory fungible-token and account model for LP-0021
//! phase 3.
//!
//! # HONESTY / LIMITATIONS — READ FIRST
//!
//! * This is a **local in-memory model**. There is no LEZ zone, no chain, no
//!   sequencer, no token program, no testnet and no persistence. Nothing is
//!   submitted or signed anywhere.
//! * `PrivateHandle` is an **opaque, non-cryptographic, clearly labelled
//!   handle**. It is an FNV-style digest of a local counter with the label
//!   `pv-handle-sim-`. It is **not** a key, not a cryptographic commitment, not
//!   a shielded address and not a zero-knowledge proof. It hides nothing and
//!   proves nothing; it exists only to test public/private account shapes.
//! * Balances are stored in plain `u128` values so the invariants can be
//!   checked locally. There are no real balances and no real users.
//! * Every transfer requires an explicit `Approval` record. Approvals may be
//!   bounded (`AmountBound::Limited`) or explicitly unbounded
//!   (`AmountBound::Unlimited`); unbounded approvals are a risk, not a feature,
//!   and are warned about by `wallet-summary`.
//! * This is **not criterion-complete for LP-0021** and is not adoption
//!   evidence of any kind.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const HEX: &[u8; 16] = b"0123456789abcdef";
const MAX_DECIMALS: u8 = 18;

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

/// Opaque account identifier. Only the local `acct-N` form is ever displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountId(u64);

impl AccountId {
    /// Construct an id from a local serial (used by the ledger and fixtures).
    pub const fn new(serial: u64) -> Self {
        Self(serial)
    }

    /// Process-local serial form.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Display for AccountId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "acct-{}", self.0)
    }
}

/// Account kind. `Private` means "modelled with an opaque handle" only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccountKind {
    /// Transparent simulated account.
    Public,
    /// Simulated account addressed only by an opaque non-crypto handle.
    Private,
}

impl AccountKind {
    /// Short stable label for output and events.
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

/// An account identifier plus its kind. Both must match ledger state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Account {
    id: AccountId,
    kind: AccountKind,
}

impl Account {
    /// Combine an id and a kind into an account reference.
    pub const fn new(id: AccountId, kind: AccountKind) -> Self {
        Self { id, kind }
    }

    /// Opaque account id.
    pub const fn id(self) -> AccountId {
        self.id
    }

    /// Simulated account kind.
    pub const fn kind(self) -> AccountKind {
        self.kind
    }
}

impl Display for Account {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.kind, self.id)
    }
}

/// Mint identifier for a simulated fungible token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MintId(u64);

impl MintId {
    /// Construct a mint id from a local serial.
    pub const fn new(serial: u64) -> Self {
        Self(serial)
    }

    /// Local serial form.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Display for MintId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "mint-{}", self.0)
    }
}

/// Asset identifier: either the simulated native asset or a mint id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetId {
    /// The simulated native asset.
    Native,
    /// A fungible token identified by a simulated mint id.
    Mint(MintId),
}

impl AssetId {
    /// Native asset id.
    pub const fn native() -> Self {
        Self::Native
    }

    /// Fungible token asset id.
    pub const fn mint(mint: MintId) -> Self {
        Self::Mint(mint)
    }

    /// Stable textual id for output and events.
    pub fn id(&self) -> String {
        match self {
            Self::Native => "native".to_owned(),
            Self::Mint(mint) => format!("mint-{}", mint.as_u64()),
        }
    }
}

impl Display for AssetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

/// Metadata for a simulated fungible token mint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenMeta {
    mint: MintId,
    decimals: u8,
    symbol: String,
}

impl TokenMeta {
    /// Build metadata. Empty symbols and more than 18 decimals are rejected;
    /// the decimal cap keeps local unit formatting free of overflow.
    pub fn new(mint: MintId, decimals: u8, symbol: &str) -> Result<Self, LedgerError> {
        if decimals > MAX_DECIMALS {
            return Err(LedgerError::DecimalsTooLarge(decimals));
        }
        let symbol = symbol.trim();
        if symbol.is_empty() {
            return Err(LedgerError::EmptyLabel);
        }
        Ok(Self {
            mint,
            decimals,
            symbol: symbol.to_owned(),
        })
    }

    /// Mint id.
    pub const fn mint(&self) -> MintId {
        self.mint
    }

    /// Number of decimal units.
    pub const fn decimals(&self) -> u8 {
        self.decimals
    }

    /// Token symbol.
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}

/// An **opaque, non-cryptographic** handle for a simulated private account.
///
/// HONESTY: this is an FNV-style digest of a local counter, labelled
/// `pv-handle-sim-`. It is not a key, a real shielded address, a cryptographic
/// commitment or a proof. It hides nothing; it only allows private-account
/// account-shape tests. Errors and logs never print this value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrivateHandle {
    serial: u64,
    digest: [u8; 8],
}

impl PrivateHandle {
    fn new(serial: u64) -> Self {
        let mut payload = Vec::with_capacity(48);
        payload.extend_from_slice(b"lp0021-phase3-private-handle-v1-not-crypto");
        payload.extend_from_slice(&serial.to_le_bytes());
        Self {
            serial,
            digest: fnv64(&payload, FNV_OFFSET).to_le_bytes(),
        }
    }

    /// Hex form of the non-cryptographic handle payload.
    pub fn as_hex(&self) -> String {
        bytes_to_hex(&self.digest)
    }
}

impl Display for PrivateHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "pv-handle-sim-{:08x}-{}", self.serial, self.as_hex())
    }
}

/// One account's balance in one asset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Balance {
    pub account: AccountId,
    pub asset: AssetId,
    pub amount: u128,
}

impl Balance {
    /// Construct a balance observation.
    pub const fn new(account: AccountId, asset: AssetId, amount: u128) -> Self {
        Self {
            account,
            asset,
            amount,
        }
    }

    /// Sum observations for one asset across accounts. An overflow is an error
    /// rather than a wrapped total.
    pub fn total_for_asset(balances: &[Balance], asset: &AssetId) -> Result<u128, LedgerError> {
        let mut total: u128 = 0;
        for balance in balances {
            if &balance.asset == asset {
                total = total
                    .checked_add(balance.amount)
                    .ok_or(LedgerError::Overflow)?;
            }
        }
        Ok(total)
    }
}

/// Identifier of an explicit approval record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApprovalId(u64);

impl ApprovalId {
    /// Construct an approval id from a local serial.
    pub const fn new(serial: u64) -> Self {
        Self(serial)
    }

    /// Local serial form.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Display for ApprovalId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "apr-{}", self.0)
    }
}

/// Program identifier. A plain non-empty label, not a key or address.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramId(String);

impl ProgramId {
    /// Build a program label.
    pub fn new(raw: &str) -> Result<Self, LedgerError> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(LedgerError::EmptyLabel);
        }
        Ok(Self(raw.to_owned()))
    }

    /// Label text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ProgramId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Who is allowed to move funds under an approval.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Spender {
    /// Another simulated account must be the recipient.
    Account(AccountId),
    /// A labelled local program acts as operator for any recipient.
    Program(ProgramId),
}

impl Display for Spender {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Account(account) => write!(f, "account {account}"),
            Self::Program(program) => write!(f, "program {program}"),
        }
    }
}

/// Bound on an approval. `Unlimited` is permitted by the model but is a risk
/// and must be surfaced explicitly to users by callers such as
/// `wallet-summary`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmountBound {
    /// At most this many base units may be spent in total.
    Limited(u128),
    /// No local spending cap. Deliberately allowed only to model a real risk.
    Unlimited,
}

impl Display for AmountBound {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limited(amount) => write!(f, "{amount}"),
            Self::Unlimited => write!(f, "unlimited"),
        }
    }
}

impl AmountBound {
    /// Bounded allowance constructor.
    pub const fn limited(amount: u128) -> Self {
        Self::Limited(amount)
    }

    /// Explicitly unbounded allowance constructor.
    pub const fn unlimited() -> Self {
        Self::Unlimited
    }

    /// Whether this bound has no cap.
    pub const fn is_unlimited(self) -> bool {
        matches!(self, Self::Unlimited)
    }

    /// Remaining base units after `used` has already been spent.
    pub fn remaining_after(self, used: u128) -> u128 {
        match self {
            Self::Limited(approved) => approved.saturating_sub(used),
            Self::Unlimited => u128::MAX,
        }
    }

    /// Whether `amount` more can be spent without exceeding the approved bound.
    pub fn covers(self, used: u128, amount: u128) -> bool {
        match self {
            Self::Limited(approved) => used
                .checked_add(amount)
                .is_some_and(|total| total <= approved),
            Self::Unlimited => true,
        }
    }
}

/// Request to create an approval record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalRequest {
    pub owner: Account,
    pub spender: Spender,
    pub asset: AssetId,
    pub bound: AmountBound,
    /// Must be true for transfers between a public and a private account.
    pub allow_cross_kind: bool,
}

/// Stored approval record. `used` never exceeds a limited bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Approval {
    id: ApprovalId,
    request: ApprovalRequest,
    used: u128,
}

impl Approval {
    /// Approval id.
    pub const fn id(&self) -> ApprovalId {
        self.id
    }

    /// Original request.
    pub fn request(&self) -> &ApprovalRequest {
        &self.request
    }

    /// Amount already spent under this approval.
    pub const fn used(&self) -> u128 {
        self.used
    }
}

impl Display for Approval {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} for {} from {}",
            self.id, self.request.asset, self.request.spender
        )
    }
}

/// Current allowance state for an approval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allowance {
    id: ApprovalId,
    bound: AmountBound,
    used: u128,
}

impl Allowance {
    /// Build an allowance view.
    pub const fn new(id: ApprovalId, bound: AmountBound, used: u128) -> Self {
        Self { id, bound, used }
    }

    /// Approval id.
    pub const fn id(self) -> ApprovalId {
        self.id
    }

    /// Original approved bound.
    pub const fn approved(self) -> AmountBound {
        self.bound
    }

    /// Amount already spent.
    pub const fn used(self) -> u128 {
        self.used
    }

    /// Remaining amount (`u128::MAX` for an unlimited approval).
    pub fn remaining(self) -> u128 {
        self.bound.remaining_after(self.used)
    }
}

/// A transfer between two explicitly identified accounts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub from: Account,
    pub to: Account,
    pub asset: AssetId,
    pub amount: u128,
    pub nonce: u64,
    pub approval: ApprovalId,
}

/// A transfer between two simulated private accounts, using only opaque
/// non-crypto handles for the endpoints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivateTransfer {
    pub from_handle: PrivateHandle,
    pub to_handle: PrivateHandle,
    pub asset: AssetId,
    pub amount: u128,
    pub nonce: u64,
    pub approval: ApprovalId,
}

/// Shape of a recorded transfer. No handle is ever stored in the log.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransferShape {
    Public,
    Private,
    Cross,
}

impl Display for TransferShape {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Cross => "cross",
        };
        write!(f, "{label}")
    }
}

fn transfer_shape(from: AccountKind, to: AccountKind) -> TransferShape {
    match (from, to) {
        (AccountKind::Public, AccountKind::Public) => TransferShape::Public,
        (AccountKind::Private, AccountKind::Private) => TransferShape::Private,
        _ => TransferShape::Cross,
    }
}

/// Result of applying a transfer locally.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub approval: ApprovalId,
    pub nonce: u64,
    pub event_index: usize,
}

/// Local event log. Contains no private handles and no secrets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerEvent {
    AccountOpened {
        account: AccountId,
        kind: AccountKind,
    },
    TokenRegistered {
        mint: MintId,
        decimals: u8,
        symbol: String,
    },
    Minted {
        to: AccountId,
        asset: AssetId,
        amount: u128,
    },
    Burned {
        from: AccountId,
        asset: AssetId,
        amount: u128,
    },
    Approved {
        approval: ApprovalId,
        owner: AccountId,
        spender: Spender,
        asset: AssetId,
        bound: AmountBound,
        allow_cross_kind: bool,
    },
    Transferred {
        from: AccountId,
        to: AccountId,
        asset: AssetId,
        amount: u128,
        nonce: u64,
        approval: ApprovalId,
        shape: TransferShape,
    },
}

/// Why a local ledger operation was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerError {
    UnknownAccount(AccountId),
    UnknownMint(MintId),
    DuplicateMint(MintId),
    UnknownApproval(ApprovalId),
    ApprovalOwnerMismatch {
        approval: ApprovalId,
        expected: AccountId,
        found: AccountId,
    },
    ApprovalAssetMismatch {
        approval: ApprovalId,
        expected: AssetId,
        found: AssetId,
    },
    ApprovalSpenderMismatch {
        approval: ApprovalId,
        expected: Spender,
        found: AccountId,
    },
    CrossKindApprovalMissing(ApprovalId),
    AllowanceExceeded {
        approval: ApprovalId,
        approved: AmountBound,
        used: u128,
        requested: u128,
    },
    InsufficientBalance {
        account: AccountId,
        asset: AssetId,
        available: u128,
        required: u128,
    },
    NonceAlreadyUsed {
        account: AccountId,
        nonce: u64,
    },
    NonceOutOfOrder {
        account: AccountId,
        expected: u64,
        found: u64,
    },
    SameAccount(AccountId),
    SameAccountKind(AccountKind),
    ZeroAmount,
    Overflow,
    WrongAccountKind {
        account: AccountId,
        expected: AccountKind,
        found: AccountKind,
    },
    /// Unknown private handle. Deliberately carries no handle value so error
    /// strings and logs cannot leak the handle.
    UnknownHandle,
    /// Duplicate private handle. Also deliberately carries no handle value.
    DuplicateHandle,
    EmptyLabel,
    DecimalsTooLarge(u8),
}

impl Display for LedgerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAccount(account) => write!(f, "unknown account: {account}"),
            Self::UnknownMint(mint) => write!(f, "unknown mint: {mint}"),
            Self::DuplicateMint(mint) => write!(f, "mint already registered: {mint}"),
            Self::UnknownApproval(approval) => write!(f, "no approval record: {approval}"),
            Self::ApprovalOwnerMismatch {
                approval,
                expected,
                found,
            } => write!(
                f,
                "approval {approval} belongs to {expected}, not {found}"
            ),
            Self::ApprovalAssetMismatch {
                approval,
                expected,
                found,
            } => write!(
                f,
                "approval {approval} is for {expected}, not {found}"
            ),
            Self::ApprovalSpenderMismatch {
                approval,
                expected,
                found,
            } => write!(f, "approval {approval} is for {expected}, not {found}"),
            Self::CrossKindApprovalMissing(approval) => write!(
                f,
                "approval {approval} does not explicitly allow public/private transfers"
            ),
            Self::AllowanceExceeded {
                approval,
                approved,
                used,
                requested,
            } => write!(
                f,
                "allowance exceeded for {approval}: approved {approved}, used {used}, requested {requested}"
            ),
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
            Self::SameAccount(account) => write!(f, "cannot transfer to the same account {account}"),
            Self::SameAccountKind(kind) => {
                write!(f, "both accounts are {kind}; this path requires mixed kinds")
            }
            Self::ZeroAmount => write!(f, "amount must be positive"),
            Self::Overflow => write!(f, "local balance counter overflow"),
            Self::WrongAccountKind {
                account,
                expected,
                found,
            } => write!(
                f,
                "account {account} is {found}, expected {expected}"
            ),
            Self::UnknownHandle => write!(f, "unknown opaque private account handle"),
            Self::DuplicateHandle => write!(f, "opaque private account handle already registered"),
            Self::EmptyLabel => write!(f, "label must not be empty"),
            Self::DecimalsTooLarge(decimals) => {
                write!(f, "decimals must be at most {MAX_DECIMALS}, found {decimals}")
            }
        }
    }
}

impl std::error::Error for LedgerError {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AccountState {
    kind: AccountKind,
    handle: Option<PrivateHandle>,
    balances: BTreeMap<AssetId, u128>,
    next_nonce: u64,
    used_nonces: BTreeSet<u64>,
}

/// Deterministic in-memory fungible-token ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenLedger {
    accounts: BTreeMap<AccountId, AccountState>,
    handles: BTreeMap<PrivateHandle, AccountId>,
    tokens: BTreeMap<MintId, TokenMeta>,
    approvals: BTreeMap<ApprovalId, Approval>,
    events: Vec<LedgerEvent>,
    next_account: u64,
    next_approval: u64,
}

impl Default for TokenLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenLedger {
    /// Empty ledger. First account id is `acct-1`, first approval `apr-1`.
    pub fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
            handles: BTreeMap::new(),
            tokens: BTreeMap::new(),
            approvals: BTreeMap::new(),
            events: Vec::new(),
            next_account: 1,
            next_approval: 1,
        }
    }

    /// Create a simulated account. Private accounts also receive an opaque
    /// non-crypto handle, which is stored internally and never logged.
    pub fn create_account(&mut self, kind: AccountKind) -> Result<Account, LedgerError> {
        let serial = self.next_account;
        self.next_account = serial.checked_add(1).ok_or(LedgerError::Overflow)?;
        let id = AccountId::new(serial);
        let handle = if kind == AccountKind::Private {
            Some(PrivateHandle::new(serial))
        } else {
            None
        };
        self.accounts.insert(
            id,
            AccountState {
                kind,
                handle,
                balances: BTreeMap::new(),
                next_nonce: 0,
                used_nonces: BTreeSet::new(),
            },
        );
        if let Some(handle) = handle {
            self.handles.insert(handle, id);
        }
        self.events
            .push(LedgerEvent::AccountOpened { account: id, kind });
        Ok(Account::new(id, kind))
    }

    /// Look up an account by id.
    pub fn account_info(&self, id: AccountId) -> Result<Account, LedgerError> {
        let state = self
            .accounts
            .get(&id)
            .ok_or(LedgerError::UnknownAccount(id))?;
        Ok(Account::new(id, state.kind))
    }

    /// The opaque handle of a simulated private account.
    pub fn private_handle(&self, account: Account) -> Result<PrivateHandle, LedgerError> {
        let state = self.account_state(account)?;
        if state.kind != AccountKind::Private {
            return Err(LedgerError::WrongAccountKind {
                account: account.id(),
                expected: AccountKind::Private,
                found: state.kind,
            });
        }
        state.handle.ok_or(LedgerError::UnknownHandle)
    }

    /// Resolve an opaque handle to an account without exposing the handle.
    pub fn resolve_handle(&self, handle: PrivateHandle) -> Result<Account, LedgerError> {
        let id = self
            .handles
            .get(&handle)
            .copied()
            .ok_or(LedgerError::UnknownHandle)?;
        self.account_info(id)
    }

    /// Register token metadata. Native assets need no registration.
    pub fn register_token(&mut self, meta: TokenMeta) -> Result<(), LedgerError> {
        let mint = meta.mint();
        if self.tokens.contains_key(&mint) {
            return Err(LedgerError::DuplicateMint(mint));
        }
        let decimals = meta.decimals();
        let symbol = meta.symbol().to_owned();
        self.tokens.insert(mint, meta);
        self.events.push(LedgerEvent::TokenRegistered {
            mint,
            decimals,
            symbol,
        });
        Ok(())
    }

    /// Registered metadata for a mint.
    pub fn token_meta(&self, mint: MintId) -> Result<TokenMeta, LedgerError> {
        self.tokens
            .get(&mint)
            .cloned()
            .ok_or(LedgerError::UnknownMint(mint))
    }

    /// Simulate a local mint credit. No token program is involved.
    pub fn mint(&mut self, to: Account, asset: AssetId, amount: u128) -> Result<(), LedgerError> {
        if amount == 0 {
            return Err(LedgerError::ZeroAmount);
        }
        self.check_asset(&asset)?;
        self.account_state(to)?;
        let state = self.account_state_mut(to)?;
        let balance = state.balances.entry(asset.clone()).or_insert(0);
        *balance = balance.checked_add(amount).ok_or(LedgerError::Overflow)?;
        self.events.push(LedgerEvent::Minted {
            to: to.id(),
            asset,
            amount,
        });
        Ok(())
    }

    /// Simulate a local burn debit.
    pub fn burn(&mut self, from: Account, asset: AssetId, amount: u128) -> Result<(), LedgerError> {
        if amount == 0 {
            return Err(LedgerError::ZeroAmount);
        }
        self.check_asset(&asset)?;
        self.account_state(from)?;
        let available = self.balance(from, asset.clone())?;
        if available < amount {
            return Err(LedgerError::InsufficientBalance {
                account: from.id(),
                asset,
                available,
                required: amount,
            });
        }
        let state = self.account_state_mut(from)?;
        let balance = state
            .balances
            .get_mut(&asset)
            .ok_or(LedgerError::InsufficientBalance {
                account: from.id(),
                asset: asset.clone(),
                available,
                required: amount,
            })?;
        *balance = balance.checked_sub(amount).ok_or(LedgerError::Overflow)?;
        self.events.push(LedgerEvent::Burned {
            from: from.id(),
            asset,
            amount,
        });
        Ok(())
    }

    /// Create an explicit approval record. Zero limited approvals are refused.
    pub fn approve(&mut self, request: ApprovalRequest) -> Result<ApprovalId, LedgerError> {
        self.check_asset(&request.asset)?;
        self.account_state(request.owner)?;
        match &request.spender {
            Spender::Account(account) => {
                self.account_info(*account)?;
            }
            Spender::Program(_) => {}
        }
        if request.bound == AmountBound::Limited(0) {
            return Err(LedgerError::ZeroAmount);
        }
        let serial = self.next_approval;
        self.next_approval = serial.checked_add(1).ok_or(LedgerError::Overflow)?;
        let id = ApprovalId::new(serial);
        let event = LedgerEvent::Approved {
            approval: id,
            owner: request.owner.id(),
            spender: request.spender.clone(),
            asset: request.asset.clone(),
            bound: request.bound,
            allow_cross_kind: request.allow_cross_kind,
        };
        self.approvals.insert(
            id,
            Approval {
                id,
                request,
                used: 0,
            },
        );
        self.events.push(event);
        Ok(id)
    }

    /// Current allowance for an approval.
    pub fn allowance(&self, id: ApprovalId) -> Result<Allowance, LedgerError> {
        let approval = self
            .approvals
            .get(&id)
            .ok_or(LedgerError::UnknownApproval(id))?;
        Ok(Allowance::new(id, approval.request.bound, approval.used))
    }

    /// Stored approval record.
    pub fn approval(&self, id: ApprovalId) -> Result<&Approval, LedgerError> {
        self.approvals
            .get(&id)
            .ok_or(LedgerError::UnknownApproval(id))
    }

    /// Balance of one account in one asset. Unknown accounts are errors.
    pub fn balance(&self, account: Account, asset: AssetId) -> Result<u128, LedgerError> {
        let state = self.account_state(account)?;
        Ok(state.balances.get(&asset).copied().unwrap_or(0))
    }

    /// All recorded per-asset balances of one account.
    pub fn balances(&self, account: Account) -> Result<Vec<Balance>, LedgerError> {
        let state = self.account_state(account)?;
        Ok(state
            .balances
            .iter()
            .map(|(asset, amount)| Balance::new(account.id(), asset.clone(), *amount))
            .collect())
    }

    /// Aggregate one asset across public and private accounts alike.
    pub fn aggregate(&self, accounts: &[Account], asset: AssetId) -> Result<u128, LedgerError> {
        let mut total: u128 = 0;
        for account in accounts {
            total = total
                .checked_add(self.balance(*account, asset.clone())?)
                .ok_or(LedgerError::Overflow)?;
        }
        Ok(total)
    }

    /// Whether a sender nonce has been consumed.
    pub fn used_nonce(&self, account: Account, nonce: u64) -> Result<bool, LedgerError> {
        Ok(self.account_state(account)?.used_nonces.contains(&nonce))
    }

    /// Next sequential nonce expected from an account.
    pub fn next_nonce(&self, account: Account) -> Result<u64, LedgerError> {
        Ok(self.account_state(account)?.next_nonce)
    }

    /// Public-to-public transfer. Both endpoints must be public accounts.
    pub fn transfer_public(&mut self, tx: Transfer) -> Result<Receipt, LedgerError> {
        check_kind(tx.from, AccountKind::Public)?;
        check_kind(tx.to, AccountKind::Public)?;
        self.apply_transfer(tx)
    }

    /// Private-to-private transfer using only opaque handles.
    pub fn transfer_private(&mut self, tx: PrivateTransfer) -> Result<Receipt, LedgerError> {
        let from = self.resolve_handle(tx.from_handle)?;
        let to = self.resolve_handle(tx.to_handle)?;
        check_kind(from, AccountKind::Private)?;
        check_kind(to, AccountKind::Private)?;
        self.apply_transfer(Transfer {
            from,
            to,
            asset: tx.asset,
            amount: tx.amount,
            nonce: tx.nonce,
            approval: tx.approval,
        })
    }

    /// Public-to-private or private-to-public transfer. The approval must
    /// explicitly permit cross-kind transfers.
    pub fn transfer_cross(&mut self, tx: Transfer) -> Result<Receipt, LedgerError> {
        if tx.from.kind() == tx.to.kind() {
            return Err(LedgerError::SameAccountKind(tx.from.kind()));
        }
        self.apply_transfer(tx)
    }

    /// Append-only local event log. Never contains a private handle.
    pub fn events(&self) -> &[LedgerEvent] {
        &self.events
    }

    fn account_state(&self, account: Account) -> Result<&AccountState, LedgerError> {
        let state = self
            .accounts
            .get(&account.id())
            .ok_or(LedgerError::UnknownAccount(account.id()))?;
        if state.kind != account.kind() {
            return Err(LedgerError::WrongAccountKind {
                account: account.id(),
                expected: account.kind(),
                found: state.kind,
            });
        }
        Ok(state)
    }

    fn account_state_mut(&mut self, account: Account) -> Result<&mut AccountState, LedgerError> {
        let expected = account.kind();
        let id = account.id();
        let state = self
            .accounts
            .get_mut(&id)
            .ok_or(LedgerError::UnknownAccount(id))?;
        if state.kind != expected {
            return Err(LedgerError::WrongAccountKind {
                account: id,
                expected,
                found: state.kind,
            });
        }
        Ok(state)
    }

    fn check_asset(&self, asset: &AssetId) -> Result<(), LedgerError> {
        if let AssetId::Mint(mint) = asset {
            if !self.tokens.contains_key(mint) {
                return Err(LedgerError::UnknownMint(*mint));
            }
        }
        Ok(())
    }

    fn apply_transfer(&mut self, tx: Transfer) -> Result<Receipt, LedgerError> {
        self.account_state(tx.from)?;
        self.account_state(tx.to)?;
        if tx.from.id() == tx.to.id() {
            return Err(LedgerError::SameAccount(tx.from.id()));
        }
        if tx.amount == 0 {
            return Err(LedgerError::ZeroAmount);
        }
        self.check_asset(&tx.asset)?;

        let approval = self
            .approvals
            .get(&tx.approval)
            .cloned()
            .ok_or(LedgerError::UnknownApproval(tx.approval))?;
        if approval.request.owner != tx.from {
            return Err(LedgerError::ApprovalOwnerMismatch {
                approval: tx.approval,
                expected: approval.request.owner.id(),
                found: tx.from.id(),
            });
        }
        if approval.request.asset != tx.asset {
            return Err(LedgerError::ApprovalAssetMismatch {
                approval: tx.approval,
                expected: approval.request.asset.clone(),
                found: tx.asset.clone(),
            });
        }
        if let Spender::Account(spender) = &approval.request.spender {
            if *spender != tx.to.id() {
                return Err(LedgerError::ApprovalSpenderMismatch {
                    approval: tx.approval,
                    expected: approval.request.spender.clone(),
                    found: tx.to.id(),
                });
            }
        }

        let sender = self.account_state(tx.from)?;
        if sender.used_nonces.contains(&tx.nonce) || tx.nonce < sender.next_nonce {
            return Err(LedgerError::NonceAlreadyUsed {
                account: tx.from.id(),
                nonce: tx.nonce,
            });
        }
        if tx.nonce > sender.next_nonce {
            return Err(LedgerError::NonceOutOfOrder {
                account: tx.from.id(),
                expected: sender.next_nonce,
                found: tx.nonce,
            });
        }
        let available = sender.balances.get(&tx.asset).copied().unwrap_or(0);
        if available < tx.amount {
            return Err(LedgerError::InsufficientBalance {
                account: tx.from.id(),
                asset: tx.asset.clone(),
                available,
                required: tx.amount,
            });
        }
        if !approval.request.bound.covers(approval.used, tx.amount) {
            return Err(LedgerError::AllowanceExceeded {
                approval: tx.approval,
                approved: approval.request.bound,
                used: approval.used,
                requested: tx.amount,
            });
        }
        if tx.from.kind() != tx.to.kind() && !approval.request.allow_cross_kind {
            return Err(LedgerError::CrossKindApprovalMissing(tx.approval));
        }
        let new_used = approval
            .used
            .checked_add(tx.amount)
            .ok_or(LedgerError::Overflow)?;
        let shape = transfer_shape(tx.from.kind(), tx.to.kind());
        let event_index = self.events.len();

        {
            let sender = self.account_state_mut(tx.from)?;
            let balance =
                sender
                    .balances
                    .get_mut(&tx.asset)
                    .ok_or(LedgerError::InsufficientBalance {
                        account: tx.from.id(),
                        asset: tx.asset.clone(),
                        available,
                        required: tx.amount,
                    })?;
            *balance = balance
                .checked_sub(tx.amount)
                .ok_or(LedgerError::Overflow)?;
            sender.used_nonces.insert(tx.nonce);
            sender.next_nonce = tx.nonce.checked_add(1).ok_or(LedgerError::Overflow)?;
        }
        {
            let receiver = self.account_state_mut(tx.to)?;
            let balance = receiver.balances.entry(tx.asset.clone()).or_insert(0);
            *balance = balance
                .checked_add(tx.amount)
                .ok_or(LedgerError::Overflow)?;
        }
        self.approvals
            .get_mut(&tx.approval)
            .expect("approval validated above")
            .used = new_used;
        self.events.push(LedgerEvent::Transferred {
            from: tx.from.id(),
            to: tx.to.id(),
            asset: tx.asset.clone(),
            amount: tx.amount,
            nonce: tx.nonce,
            approval: tx.approval,
            shape,
        });

        Ok(Receipt {
            approval: tx.approval,
            nonce: tx.nonce,
            event_index,
        })
    }
}

fn check_kind(account: Account, expected: AccountKind) -> Result<(), LedgerError> {
    if account.kind() == expected {
        Ok(())
    } else {
        Err(LedgerError::WrongAccountKind {
            account: account.id(),
            expected,
            found: account.kind(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINT: MintId = MintId::new(1);

    fn token() -> AssetId {
        AssetId::mint(MINT)
    }

    fn fixture() -> (TokenLedger, Account, Account, ApprovalId) {
        let mut ledger = TokenLedger::new();
        ledger
            .register_token(TokenMeta::new(MINT, 0, "TKN").unwrap())
            .unwrap();
        let alice = ledger.create_account(AccountKind::Public).unwrap();
        let bob = ledger.create_account(AccountKind::Private).unwrap();
        ledger.mint(alice, token(), 100).unwrap();
        let approval = ledger
            .approve(ApprovalRequest {
                owner: alice,
                spender: Spender::Account(bob.id()),
                asset: token(),
                bound: AmountBound::limited(50),
                allow_cross_kind: true,
            })
            .unwrap();
        (ledger, alice, bob, approval)
    }

    #[test]
    fn account_ids_are_opaque_and_kinds_distinct() {
        let mut ledger = TokenLedger::new();
        let public = ledger.create_account(AccountKind::Public).unwrap();
        let private = ledger.create_account(AccountKind::Private).unwrap();
        assert_eq!(public.id().to_string(), "acct-1");
        assert_eq!(private.id().as_u64(), 2);
        assert_eq!(public.kind(), AccountKind::Public);
        assert_eq!(private.kind(), AccountKind::Private);
        assert_eq!(private.to_string(), "private acct-2");
    }

    #[test]
    fn asset_ids_and_mint_metadata_are_distinct() {
        assert_eq!(AssetId::native().id(), "native");
        assert_eq!(token().id(), "mint-1");
        assert_ne!(AssetId::native(), token());
        let meta = TokenMeta::new(MINT, 6, " USDC ").unwrap();
        assert_eq!(meta.mint(), MINT);
        assert_eq!(meta.decimals(), 6);
        assert_eq!(meta.symbol(), "USDC");
    }

    #[test]
    fn token_meta_rejects_bad_decimals_and_empty_symbol() {
        assert_eq!(
            TokenMeta::new(MINT, 19, "TKN"),
            Err(LedgerError::DecimalsTooLarge(19))
        );
        assert_eq!(TokenMeta::new(MINT, 2, "  "), Err(LedgerError::EmptyLabel));
    }

    #[test]
    fn private_handle_is_opaque_and_labelled_non_crypto() {
        let mut ledger = TokenLedger::new();
        let account = ledger.create_account(AccountKind::Private).unwrap();
        let handle = ledger.private_handle(account).unwrap();
        let rendered = handle.to_string();
        assert!(rendered.starts_with("pv-handle-sim-"));
        assert!(rendered.contains("sim"));
        assert!(!rendered.contains(&account.id().to_string()));
        assert_eq!(ledger.resolve_handle(handle), Ok(account));
    }

    #[test]
    fn private_handle_resolution_fails_for_public_accounts() {
        let mut ledger = TokenLedger::new();
        let public = ledger.create_account(AccountKind::Public).unwrap();
        assert_eq!(
            ledger.private_handle(public),
            Err(LedgerError::WrongAccountKind {
                account: public.id(),
                expected: AccountKind::Private,
                found: AccountKind::Public,
            })
        );
    }

    #[test]
    fn unknown_handle_is_rejected() {
        let mut issuer = TokenLedger::new();
        let private = issuer.create_account(AccountKind::Private).unwrap();
        let handle = issuer.private_handle(private).unwrap();
        let other = TokenLedger::new();
        assert_eq!(
            other.resolve_handle(handle),
            Err(LedgerError::UnknownHandle)
        );
    }

    #[test]
    fn mint_and_balance_are_per_account_and_asset() {
        let (ledger, alice, bob, _) = fixture();
        assert_eq!(ledger.balance(alice, token()), Ok(100));
        assert_eq!(ledger.balance(bob, token()), Ok(0));
        assert_eq!(ledger.balance(alice, AssetId::native()), Ok(0));
        assert_eq!(ledger.balances(alice).unwrap().len(), 1);
    }

    #[test]
    fn burn_rejects_negative_balance_without_mutating_state() {
        let (mut ledger, alice, _, _) = fixture();
        assert_eq!(
            ledger.burn(alice, token(), 101),
            Err(LedgerError::InsufficientBalance {
                account: alice.id(),
                asset: token(),
                available: 100,
                required: 101,
            })
        );
        assert_eq!(ledger.balance(alice, token()), Ok(100));
        ledger.burn(alice, token(), 40).unwrap();
        assert_eq!(ledger.balance(alice, token()), Ok(60));
    }

    #[test]
    fn mint_rejects_unregistered_mint_zero_and_duplicate() {
        let mut ledger = TokenLedger::new();
        let alice = ledger.create_account(AccountKind::Public).unwrap();
        assert_eq!(
            ledger.mint(alice, token(), 1),
            Err(LedgerError::UnknownMint(MINT))
        );
        ledger
            .register_token(TokenMeta::new(MINT, 0, "TKN").unwrap())
            .unwrap();
        assert_eq!(ledger.mint(alice, token(), 0), Err(LedgerError::ZeroAmount));
        assert_eq!(
            ledger.register_token(TokenMeta::new(MINT, 0, "TKN").unwrap()),
            Err(LedgerError::DuplicateMint(MINT))
        );
    }

    #[test]
    fn approve_creates_bounded_allowance() {
        let (ledger, _, _, approval) = fixture();
        let allowance = ledger.allowance(approval).unwrap();
        assert_eq!(allowance.id(), approval);
        assert_eq!(allowance.approved(), AmountBound::limited(50));
        assert_eq!(allowance.used(), 0);
        assert_eq!(allowance.remaining(), 50);
        assert_eq!(ledger.approval(approval).unwrap().used(), 0);
    }

    #[test]
    fn allowance_cannot_exceed_approved_amount() {
        let (mut ledger, alice, bob, approval) = fixture();
        ledger
            .transfer_cross(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 30,
                nonce: 0,
                approval,
            })
            .unwrap();
        assert_eq!(ledger.allowance(approval).unwrap().remaining(), 20);
        let too_much = Transfer {
            from: alice,
            to: bob,
            asset: token(),
            amount: 21,
            nonce: 1,
            approval,
        };
        assert_eq!(
            ledger.transfer_cross(too_much),
            Err(LedgerError::AllowanceExceeded {
                approval,
                approved: AmountBound::limited(50),
                used: 30,
                requested: 21,
            })
        );
        assert_eq!(ledger.balance(alice, token()), Ok(70));
        assert_eq!(ledger.allowance(approval).unwrap().remaining(), 20);
    }

    #[test]
    fn unlimited_approval_reports_unbounded_remaining() {
        let mut ledger = TokenLedger::new();
        ledger
            .register_token(TokenMeta::new(MINT, 0, "TKN").unwrap())
            .unwrap();
        let alice = ledger.create_account(AccountKind::Public).unwrap();
        let program = ProgramId::new("P").unwrap();
        let approval = ledger
            .approve(ApprovalRequest {
                owner: alice,
                spender: Spender::Program(program),
                asset: token(),
                bound: AmountBound::unlimited(),
                allow_cross_kind: false,
            })
            .unwrap();
        let allowance = ledger.allowance(approval).unwrap();
        assert!(allowance.approved().is_unlimited());
        assert_eq!(allowance.remaining(), u128::MAX);
        assert_eq!(AmountBound::unlimited().to_string(), "unlimited".to_owned());
    }

    #[test]
    fn zero_limited_approval_is_rejected() {
        let (mut ledger, alice, bob, _) = fixture();
        assert_eq!(
            ledger.approve(ApprovalRequest {
                owner: alice,
                spender: Spender::Account(bob.id()),
                asset: token(),
                bound: AmountBound::limited(0),
                allow_cross_kind: false,
            }),
            Err(LedgerError::ZeroAmount)
        );
    }

    #[test]
    fn transfer_requires_an_existing_approval_record() {
        let (mut ledger, alice, bob, _) = fixture();
        assert_eq!(
            ledger.transfer_cross(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 1,
                nonce: 0,
                approval: ApprovalId::new(999),
            }),
            Err(LedgerError::UnknownApproval(ApprovalId::new(999)))
        );
        assert_eq!(ledger.balance(alice, token()), Ok(100));
        assert_eq!(ledger.balance(bob, token()), Ok(0));
    }

    #[test]
    fn approval_must_match_owner_asset_and_spender() {
        let (mut ledger, alice, bob, approval) = fixture();
        let third = ledger.create_account(AccountKind::Public).unwrap();
        let wrong_owner = Transfer {
            from: third,
            to: alice,
            asset: token(),
            amount: 1,
            nonce: 0,
            approval,
        };
        assert!(matches!(
            ledger.transfer_public(wrong_owner),
            Err(LedgerError::ApprovalOwnerMismatch { .. })
        ));
        let wrong_asset = Transfer {
            from: alice,
            to: bob,
            asset: AssetId::native(),
            amount: 1,
            nonce: 0,
            approval,
        };
        assert!(matches!(
            ledger.transfer_cross(wrong_asset),
            Err(LedgerError::ApprovalAssetMismatch { .. })
        ));
        let wrong_spender = Transfer {
            from: alice,
            to: third,
            asset: token(),
            amount: 1,
            nonce: 0,
            approval,
        };
        assert!(matches!(
            ledger.transfer_public(wrong_spender),
            Err(LedgerError::ApprovalSpenderMismatch { .. })
        ));
    }

    #[test]
    fn nonce_replay_is_rejected() {
        let (mut ledger, alice, bob, approval) = fixture();
        let tx = Transfer {
            from: alice,
            to: bob,
            asset: token(),
            amount: 5,
            nonce: 0,
            approval,
        };
        ledger.transfer_cross(tx.clone()).unwrap();
        assert!(ledger.used_nonce(alice, 0).unwrap());
        assert_eq!(
            ledger.transfer_cross(tx),
            Err(LedgerError::NonceAlreadyUsed {
                account: alice.id(),
                nonce: 0,
            })
        );
        assert_eq!(ledger.next_nonce(alice), Ok(1));
    }

    #[test]
    fn out_of_order_nonce_is_rejected() {
        let (mut ledger, alice, bob, approval) = fixture();
        assert_eq!(
            ledger.transfer_cross(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 1,
                nonce: 3,
                approval,
            }),
            Err(LedgerError::NonceOutOfOrder {
                account: alice.id(),
                expected: 0,
                found: 3,
            })
        );
    }

    #[test]
    fn nonces_are_per_account() {
        let (mut ledger, alice, bob, approval) = fixture();
        let second = ledger.create_account(AccountKind::Public).unwrap();
        ledger.mint(second, token(), 10).unwrap();
        let approval_b = ledger
            .approve(ApprovalRequest {
                owner: second,
                spender: Spender::Account(bob.id()),
                asset: token(),
                bound: AmountBound::limited(10),
                allow_cross_kind: true,
            })
            .unwrap();
        ledger
            .transfer_cross(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 1,
                nonce: 0,
                approval,
            })
            .unwrap();
        ledger
            .transfer_cross(Transfer {
                from: second,
                to: bob,
                asset: token(),
                amount: 1,
                nonce: 0,
                approval: approval_b,
            })
            .unwrap();
        assert_eq!(ledger.next_nonce(alice), Ok(1));
        assert_eq!(ledger.next_nonce(second), Ok(1));
    }

    #[test]
    fn public_to_public_transfer_moves_balances_and_records_shape() {
        let (mut ledger, alice, private, _) = fixture();
        let bob = ledger.create_account(AccountKind::Public).unwrap();
        let approval = ledger
            .approve(ApprovalRequest {
                owner: alice,
                spender: Spender::Account(bob.id()),
                asset: token(),
                bound: AmountBound::limited(50),
                allow_cross_kind: false,
            })
            .unwrap();
        let receipt = ledger
            .transfer_public(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 12,
                nonce: 0,
                approval,
            })
            .unwrap();
        assert_eq!(receipt.nonce, 0);
        assert_eq!(ledger.balance(alice, token()), Ok(88));
        assert_eq!(ledger.balance(bob, token()), Ok(12));
        assert_eq!(ledger.balance(private, token()), Ok(0));
        match ledger.events().last().unwrap() {
            LedgerEvent::Transferred { shape, .. } => {
                assert_eq!(*shape, TransferShape::Public)
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn private_to_private_transfer_uses_opaque_handles() {
        let mut ledger = TokenLedger::new();
        ledger
            .register_token(TokenMeta::new(MINT, 0, "TKN").unwrap())
            .unwrap();
        let alice = ledger.create_account(AccountKind::Private).unwrap();
        let bob = ledger.create_account(AccountKind::Private).unwrap();
        let from_handle = ledger.private_handle(alice).unwrap();
        let to_handle = ledger.private_handle(bob).unwrap();
        ledger.mint(alice, token(), 20).unwrap();
        let approval = ledger
            .approve(ApprovalRequest {
                owner: alice,
                spender: Spender::Account(bob.id()),
                asset: token(),
                bound: AmountBound::limited(20),
                allow_cross_kind: false,
            })
            .unwrap();
        ledger
            .transfer_private(PrivateTransfer {
                from_handle,
                to_handle,
                asset: token(),
                amount: 7,
                nonce: 0,
                approval,
            })
            .unwrap();
        assert_eq!(ledger.balance(alice, token()), Ok(13));
        assert_eq!(ledger.balance(bob, token()), Ok(7));
        match ledger.events().last().unwrap() {
            LedgerEvent::Transferred { shape, .. } => {
                assert_eq!(*shape, TransferShape::Private)
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn private_handle_never_appears_in_error_strings_or_events() {
        let mut ledger = TokenLedger::new();
        let private = ledger.create_account(AccountKind::Private).unwrap();
        let handle = ledger.private_handle(private).unwrap();
        let rendered_handle = handle.to_string();
        let error = LedgerError::UnknownHandle.to_string();
        assert!(!error.contains(&rendered_handle));
        assert!(!error.contains("pv-handle-sim"));
        let events = format!("{:?}", ledger.events());
        assert!(!events.contains(&rendered_handle));
        assert!(!events.contains("pv-handle-sim"));
    }

    #[test]
    fn cross_kind_transfer_requires_explicit_cross_approval() {
        let (mut ledger, alice, bob, _) = fixture();
        let without_cross = ledger
            .approve(ApprovalRequest {
                owner: alice,
                spender: Spender::Account(bob.id()),
                asset: token(),
                bound: AmountBound::limited(10),
                allow_cross_kind: false,
            })
            .unwrap();
        assert_eq!(
            ledger.transfer_cross(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 1,
                nonce: 0,
                approval: without_cross,
            }),
            Err(LedgerError::CrossKindApprovalMissing(without_cross))
        );
        assert_eq!(ledger.balance(alice, token()), Ok(100));
    }

    #[test]
    fn cross_kind_transfer_succeeds_with_explicit_cross_approval() {
        let (mut ledger, alice, bob, approval) = fixture();
        ledger
            .transfer_cross(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 9,
                nonce: 0,
                approval,
            })
            .unwrap();
        assert_eq!(ledger.balance(alice, token()), Ok(91));
        assert_eq!(ledger.balance(bob, token()), Ok(9));
        match ledger.events().last().unwrap() {
            LedgerEvent::Transferred { shape, .. } => {
                assert_eq!(*shape, TransferShape::Cross)
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn transfer_cross_rejects_same_kind_endpoints() {
        let (mut ledger, alice, _, approval) = fixture();
        let another = ledger.create_account(AccountKind::Public).unwrap();
        assert_eq!(
            ledger.transfer_cross(Transfer {
                from: alice,
                to: another,
                asset: token(),
                amount: 1,
                nonce: 0,
                approval,
            }),
            Err(LedgerError::SameAccountKind(AccountKind::Public))
        );
    }

    #[test]
    fn transfer_public_rejects_private_endpoint() {
        let (mut ledger, alice, bob, approval) = fixture();
        assert_eq!(
            ledger.transfer_public(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 1,
                nonce: 0,
                approval,
            }),
            Err(LedgerError::WrongAccountKind {
                account: bob.id(),
                expected: AccountKind::Public,
                found: AccountKind::Private,
            })
        );
    }

    #[test]
    fn balance_aggregation_covers_public_and_private_accounts() {
        let (mut ledger, alice, bob, approval) = fixture();
        ledger
            .transfer_cross(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 40,
                nonce: 0,
                approval,
            })
            .unwrap();
        assert_eq!(ledger.balance(alice, token()), Ok(60));
        assert_eq!(ledger.balance(bob, token()), Ok(40));
        assert_eq!(ledger.aggregate(&[alice, bob], token()), Ok(100));
    }

    #[test]
    fn balance_struct_totals_only_matching_asset() {
        let balances = vec![
            Balance::new(AccountId::new(1), token(), 10),
            Balance::new(AccountId::new(2), AssetId::native(), 5),
            Balance::new(AccountId::new(3), token(), 7),
        ];
        assert_eq!(Balance::total_for_asset(&balances, &token()), Ok(17));
        assert_eq!(
            Balance::total_for_asset(&balances, &AssetId::native()),
            Ok(5)
        );
    }

    #[test]
    fn zero_amount_and_self_transfer_are_rejected() {
        let (mut ledger, alice, bob, approval) = fixture();
        let zero = Transfer {
            from: alice,
            to: bob,
            asset: token(),
            amount: 0,
            nonce: 0,
            approval,
        };
        assert_eq!(
            ledger.transfer_cross(zero.clone()),
            Err(LedgerError::ZeroAmount)
        );
        let self_transfer = Transfer {
            to: alice,
            amount: 1,
            ..zero
        };
        assert_eq!(
            ledger.transfer_public(self_transfer),
            Err(LedgerError::SameAccount(alice.id()))
        );
    }

    #[test]
    fn program_spender_can_send_to_any_account() {
        let (mut ledger, alice, bob, _) = fixture();
        let program = ProgramId::new("P").unwrap();
        let wide = ledger
            .approve(ApprovalRequest {
                owner: alice,
                spender: Spender::Program(program),
                asset: token(),
                bound: AmountBound::limited(10),
                allow_cross_kind: true,
            })
            .unwrap();
        ledger
            .transfer_cross(Transfer {
                from: alice,
                to: bob,
                asset: token(),
                amount: 4,
                nonce: 0,
                approval: wide,
            })
            .unwrap();
        assert_eq!(ledger.balance(bob, token()), Ok(4));
    }

    #[test]
    fn allowance_bounds_are_exposed_in_events() {
        let (ledger, _, _, _) = fixture();
        let found = ledger.events().iter().any(|event| {
            matches!(
                event,
                LedgerEvent::Approved {
                    bound: AmountBound::Limited(50),
                    allow_cross_kind: true,
                    ..
                }
            )
        });
        assert!(found);
    }
}
