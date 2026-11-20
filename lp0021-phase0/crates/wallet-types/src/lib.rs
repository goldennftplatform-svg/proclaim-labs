// SPDX-License-Identifier: MIT AND Apache-2.0

//! Core domain types for the LP-0021 wallet SDK (Phase 0).
//!
//! This crate is deliberately dependency-light: only `serde` is used, and all
//! types are `Debug + Clone + PartialEq + Serialize + Deserialize`. Nothing in
//! here talks to a chain, a network, or a keystore.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Small helper used by the keystore and the core to render byte material.
pub mod hexutil {
    pub fn encode(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0x0f) as usize] as char);
        }
        out
    }

    pub fn decode(s: &str) -> Option<Vec<u8>> {
        if !s.len().is_multiple_of(2) {
            return None;
        }
        let mut out = Vec::with_capacity(s.len() / 2);
        let bytes = s.as_bytes();
        for pair in bytes.as_chunks::<2>().0 {
            let hi = (pair[0] as char).to_digit(16)?;
            let lo = (pair[1] as char).to_digit(16)?;
            out.push(((hi << 4) | lo) as u8);
        }
        Some(out)
    }
}

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(AccountId);
string_id!(KeyHandle);
string_id!(DappId);
string_id!(SessionId);
string_id!(ProgramId);
string_id!(RequestId);
string_id!(ActorId);
string_id!(Mint);
string_id!(ZoneId);
string_id!(ApprovalToken);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AccountKind {
    Public,
    Shielded,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    AccountAccess,
    ReadBalances,
    ProposeTransaction,
    GetVerificationStatus,
}

/// A fungible LEZ asset. NFTs are explicitly out of scope for LP-0021.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub program_id: ProgramId,
    pub mint: Mint,
    pub amount: u128,
    pub decimals: u8,
}

/// Input to `wallet_proposeTransaction` (plan section 3.3).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TransactionRequest {
    pub dapp_id: DappId,
    pub session: SessionId,
    pub program_id: ProgramId,
    pub message: Vec<u8>,
    pub declared_actors: Vec<ActorId>,
    pub fee_payer: AccountId,
    pub value: Option<Asset>,
}

/// Registry metadata resolved through the `ProgramRegistry` adapter.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProgramMeta {
    pub program_id: ProgramId,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub idl_pointer: Option<String>,
    pub code_pointer: Option<String>,
    pub registered_at: Option<String>,
    pub deployer_signed: bool,
}

/// Source-verification badge shown in the approval prompt (plan section 3.4).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum VerificationStatus {
    Unregistered,
    RegisteredUnverified,
    LocallyVerified {
        source_commit: String,
        image_id: String,
    },
    DeployerClaimUnverified,
    Mismatch,
}

impl VerificationStatus {
    pub fn badge(&self) -> &'static str {
        match self {
            Self::Unregistered => "unverified / unknown program",
            Self::RegisteredUnverified => "registered, source unverified",
            Self::LocallyVerified { .. } => "locally source-verified",
            Self::DeployerClaimUnverified => "deployer claim unverified",
            Self::Mismatch => "source mismatch — high risk",
        }
    }
}

/// A token approval granted by the transaction (the "unlimited approval" case
/// is called out explicitly in the prompt).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ApprovalGrant {
    pub owner: AccountId,
    pub spender: String,
    pub mint: Mint,
    pub amount: Option<u128>,
    pub unlimited: bool,
}

/// An asset movement produced by simulation.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AssetMovement {
    pub from: String,
    pub to: String,
    pub program_id: ProgramId,
    pub mint: Mint,
    pub amount: u128,
}

/// Structured output of IDL decode + simulate (plan section 3.4 step 3).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SimulationResult {
    pub effects_available: bool,
    pub asset: Option<Asset>,
    pub destination: Option<String>,
    pub approvals_granted: Vec<ApprovalGrant>,
    pub approvals_moved: Vec<AssetMovement>,
    pub unlimited_approval: bool,
    pub gas_estimate: Option<u64>,
    pub gas_used: Option<u64>,
    pub actors_declared: Vec<ActorId>,
    pub signers_required: Vec<AccountId>,
    pub applies: Option<bool>,
    pub fee_estimate: Option<u128>,
    pub notes: Vec<String>,
}

impl SimulationResult {
    /// An "unknown" simulation: used when the IDL lookup fails. It never
    /// asserts that effects are safe; it only carries the raw calldata fallback.
    pub fn degraded() -> Self {
        Self {
            effects_available: false,
            asset: None,
            destination: None,
            approvals_granted: Vec::new(),
            approvals_moved: Vec::new(),
            unlimited_approval: false,
            gas_estimate: None,
            gas_used: None,
            actors_declared: Vec::new(),
            signers_required: Vec::new(),
            applies: None,
            fee_estimate: None,
            notes: Vec::new(),
        }
    }

    pub fn ok() -> Self {
        Self {
            effects_available: true,
            ..Self::degraded()
        }
    }
}

/// Human-readable summary shown at the approval prompt. This is the shape
/// described in plan section 3.3 (fields after `EffectsSummary fields:`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EffectsSummary {
    pub requesting_dapp: DappId,
    pub session: SessionId,
    pub target_account: AccountId,
    pub target_zone: ZoneId,
    pub program_id: ProgramId,
    pub program_name: Option<String>,
    pub program_version: Option<String>,
    pub program_author: Option<String>,
    pub asset: Option<Asset>,
    pub amount: Option<u128>,
    pub destination: Option<String>,
    pub approvals_granted: Vec<ApprovalGrant>,
    pub approvals_moved: Vec<AssetMovement>,
    pub unlimited_approval: bool,
    pub gas_estimate: Option<u64>,
    pub gas_used: Option<u64>,
    pub verification_status: VerificationStatus,
    pub verification_detail: Option<String>,
    pub warnings: Vec<String>,
    pub raw_calldata_hex: String,
    pub effects_available: bool,
    pub actors_declared: Vec<ActorId>,
    pub signers_required: Vec<AccountId>,
    pub applies: Option<bool>,
    pub fee_estimate: Option<u128>,
}

impl EffectsSummary {
    /// Lines intended for the QML approval dialog. Never hides warnings.
    pub fn to_human_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!("Requesting dApp: {}", self.requesting_dapp));
        lines.push(format!(
            "Account: {} ({})",
            self.target_account, self.target_zone
        ));
        let program = match (
            &self.program_name,
            &self.program_version,
            &self.program_author,
        ) {
            (Some(name), Some(version), Some(author)) => {
                format!("{} v{} by {}", name, version, author)
            }
            (Some(name), _, _) => name.clone(),
            _ => format!("unregistered program {}", self.program_id),
        };
        lines.push(format!("Program: {}", program));
        lines.push(format!(
            "Verification: {}",
            self.verification_status.badge()
        ));
        if let Some(detail) = &self.verification_detail {
            lines.push(format!("Verification detail: {}", detail));
        }
        if self.effects_available {
            match (&self.asset, self.amount) {
                (Some(asset), Some(amount)) => lines.push(format!(
                    "Asset: {} {} (decimals {}) on {}",
                    amount, asset.mint, asset.decimals, asset.program_id
                )),
                (Some(asset), None) => {
                    lines.push(format!("Asset: {} on {}", asset.mint, asset.program_id))
                }
                _ => lines.push("Asset: none declared".to_owned()),
            }
            if let Some(destination) = &self.destination {
                lines.push(format!("Destination: {}", destination));
            }
            for approval in &self.approvals_granted {
                if approval.unlimited {
                    lines.push(format!(
                        "UNLIMITED APPROVAL: {} may spend any amount of {} (owner {})",
                        approval.spender, approval.mint, approval.owner
                    ));
                } else {
                    lines.push(format!(
                        "Approval: {} may spend {:?} of {} (owner {})",
                        approval.spender, approval.amount, approval.mint, approval.owner
                    ));
                }
            }
            for movement in &self.approvals_moved {
                lines.push(format!(
                    "Transfer: {} {} from {} to {} via {}",
                    movement.amount, movement.mint, movement.from, movement.to, movement.program_id
                ));
            }
            if let Some(cycles) = self.gas_estimate {
                lines.push(format!("Estimated cycles: {}", cycles));
            }
            if let Some(cycles) = self.gas_used {
                lines.push(format!("Cycles used (simulated): {}", cycles));
            }
            if let Some(fee) = self.fee_estimate {
                lines.push(format!("Estimated fee: {}", fee));
            }
            if !self.actors_declared.is_empty() {
                lines.push(format!("Declared actors: {:?}", self.actors_declared));
            }
            if !self.signers_required.is_empty() {
                lines.push(format!("Signers required: {:?}", self.signers_required));
            }
            if let Some(applies) = self.applies {
                lines.push(format!(
                    "Simulation: {}",
                    if applies { "applies" } else { "reverts" }
                ));
            }
        } else {
            lines.push("Cannot render effects — IDL unavailable; raw calldata only".to_owned());
        }
        for warning in &self.warnings {
            lines.push(format!("WARNING: {}", warning));
        }
        lines.push(format!("Raw calldata: 0x{}", self.raw_calldata_hex));
        lines
    }
}

/// The approval state machine (plan architecture decision 3: enforced in the
/// core module before any sign/submit).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ApprovalState {
    Proposed,
    AwaitingApproval {
        effects: EffectsSummary,
    },
    Approved {
        effects: EffectsSummary,
        token: ApprovalToken,
    },
    Rejected {
        reason: Option<String>,
    },
    Signed {
        effects: EffectsSummary,
        token: ApprovalToken,
        tx: SignedTransaction,
    },
    Submitted {
        tx: SignedTransaction,
    },
    Failed {
        reason: String,
    },
}

impl ApprovalState {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Proposed => "Proposed",
            Self::AwaitingApproval { .. } => "AwaitingApproval",
            Self::Approved { .. } => "Approved",
            Self::Rejected { .. } => "Rejected",
            Self::Signed { .. } => "Signed",
            Self::Submitted { .. } => "Submitted",
            Self::Failed { .. } => "Failed",
        }
    }

    pub fn effects(&self) -> Option<&EffectsSummary> {
        match self {
            Self::AwaitingApproval { effects }
            | Self::Approved { effects, .. }
            | Self::Signed { effects, .. } => Some(effects),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SignedTransaction {
    pub request_id: RequestId,
    pub account: AccountId,
    pub signature: Vec<u8>,
    pub tx_bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SubmissionReceipt {
    pub request_id: RequestId,
    pub accepted: bool,
    pub signature_hex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Denied {
    NoSession,
    CapabilityNotGranted(Capability),
    AccountMismatch,
    UserRejected,
    InvalidState(String),
    Policy(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    NotFound(ProgramId),
    Unavailable(String),
    Malformed(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum IdlError {
    Unavailable(ProgramId),
    DecodeFailed(String),
    SimulationFailed(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum KeystoreError {
    Locked,
    AlreadyUnlocked,
    InvalidPassphrase,
    CorruptStore(String),
    KeyNotFound(KeyHandle),
    Io(String),
    Crypto(String),
    Unsupported,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum WalletError {
    UnknownRequest(RequestId),
    InvalidState {
        request_id: RequestId,
        expected: String,
        actual: String,
    },
    NotApproved(RequestId),
    ApprovalTokenMismatch(RequestId),
    Authorization(Denied),
    Registry(RegistryError),
    Idl(IdlError),
    Keystore(KeystoreError),
    Signer(String),
}

macro_rules! impl_error {
    ($t:ty) => {
        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:?}", self)
            }
        }
        impl std::error::Error for $t {}
    };
}

impl_error!(Denied);
impl_error!(RegistryError);
impl_error!(IdlError);
impl_error!(KeystoreError);
impl_error!(WalletError);

#[cfg(test)]
mod tests {
    use super::*;

    fn effects() -> EffectsSummary {
        EffectsSummary {
            requesting_dapp: DappId::new("dapp.example"),
            session: SessionId::new("s-1"),
            target_account: AccountId::new("acct-1"),
            target_zone: ZoneId::new("lez-testnet-0.4"),
            program_id: ProgramId::new("prog-1"),
            program_name: Some("token".to_owned()),
            program_version: Some("1.0".to_owned()),
            program_author: Some("logos".to_owned()),
            asset: Some(Asset {
                program_id: ProgramId::new("token-program"),
                mint: Mint::new("mint-1"),
                amount: 250,
                decimals: 6,
            }),
            amount: Some(250),
            destination: Some("acct-2".to_owned()),
            approvals_granted: vec![ApprovalGrant {
                owner: AccountId::new("acct-1"),
                spender: "spender-1".to_owned(),
                mint: Mint::new("mint-1"),
                amount: None,
                unlimited: true,
            }],
            approvals_moved: Vec::new(),
            unlimited_approval: true,
            gas_estimate: Some(12),
            gas_used: None,
            verification_status: VerificationStatus::RegisteredUnverified,
            verification_detail: None,
            warnings: vec!["unlimited approval".to_owned()],
            raw_calldata_hex: "deadbeef".to_owned(),
            effects_available: true,
            actors_declared: Vec::new(),
            signers_required: vec![AccountId::new("acct-1")],
            applies: Some(true),
            fee_estimate: Some(3),
        }
    }

    #[test]
    fn types_roundtrip_through_serde_json() {
        // serde_json is only a dev-dependency; the types themselves just derive serde.
        let e = effects();
        let json = serde_json::to_string(&e).unwrap();
        let back: EffectsSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(e, back);

        let state = ApprovalState::AwaitingApproval { effects: e };
        let json = serde_json::to_string(&state).unwrap();
        let back: ApprovalState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.name(), back.name());
    }

    #[test]
    fn human_lines_call_out_unlimited_approval_and_verification() {
        let lines = effects().to_human_lines();
        assert!(lines.iter().any(|l| l.contains("UNLIMITED APPROVAL")));
        assert!(lines.iter().any(|l| l.contains("Verification:")));
        assert!(lines.iter().any(|l| l.contains("Destination: acct-2")));
        assert!(lines.iter().any(|l| l.contains("0xdeadbeef")));
    }

    #[test]
    fn degraded_effects_show_raw_calldata_fallback() {
        let mut e = effects();
        e.effects_available = false;
        let lines = e.to_human_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("Cannot render effects — IDL unavailable")));
    }

    #[test]
    fn hexutil_roundtrip() {
        assert_eq!(
            hexutil::decode(&hexutil::encode(&[0x00, 0xab, 0xff])),
            Some(vec![0x00, 0xab, 0xff])
        );
        assert_eq!(hexutil::decode("abc"), None);
    }

    #[test]
    fn approval_state_exposes_effects_only_when_relevant() {
        assert_eq!(ApprovalState::Proposed.effects(), None);
        let e = effects();
        let state = ApprovalState::Approved {
            effects: e.clone(),
            token: ApprovalToken::new("t-1"),
        };
        assert_eq!(state.effects(), Some(&e));
        assert_eq!(state.name(), "Approved");
        assert_eq!(
            VerificationStatus::Mismatch.badge(),
            "source mismatch — high risk"
        );
    }
}
