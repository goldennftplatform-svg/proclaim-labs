// SPDX-License-Identifier: MIT AND Apache-2.0
//! LP-0021 Phase 1 local vertical slice: zero-dependency CLI plus an
//! approval-gated wallet runtime.
//!
//! The runtime reimplements the small, needed surfaces of the phase-0 approval
//! state machine and the deny-by-default consent store locally. There are no
//! path dependencies on other staged crates and no third-party crates.
//!
//! HONESTY: this is an in-memory demo. There is no LEZ zone, no sequencer, no
//! testnet 0.4, no real ZK/private pool, no keystore, and no disk persistence.

#![forbid(unsafe_code)]

pub mod approval;
pub mod args;
pub mod consent;
pub mod dispatch;
pub mod runtime;

pub use approval::{
    ApprovalAuthorizer, ApprovalState, ApprovalToken, Denied, EffectsSummary, ExplicitApprover,
    ProposalBinding, RequestId, SessionId,
};
pub use args::{parse_command, parse_commands, Command, ParseError};
pub use consent::{
    Capability, ConsentError, ConsentGate, DappId, PermissionStore, Scope, StoreGate,
};
pub use dispatch::{dispatch, CliError};
pub use runtime::{
    demo_runtime, MockSignature, MockSigner, SendOutcome, SignerAdapter, WalletError, WalletRuntime,
};
