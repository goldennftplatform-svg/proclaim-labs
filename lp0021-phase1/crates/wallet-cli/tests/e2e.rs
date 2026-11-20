// SPDX-License-Identifier: MIT AND Apache-2.0
//! End-to-end integration tests for the local vertical slice.

use std::cell::RefCell;
use std::rc::Rc;

use wallet_chain::{AccountId, AccountKind, Asset, Ledger};
use wallet_cli::approval::{ApprovalState, ExplicitApprover};
use wallet_cli::consent::{Capability, PermissionStore, Scope, StoreGate};
use wallet_cli::runtime::{demo_runtime, MockSigner, WalletError, WalletRuntime};
use wallet_cli::ProposalBinding;

fn bare_runtime() -> WalletRuntime {
    let mut ledger = Ledger::new();
    let alice = ledger.create_account(AccountKind::Public, 50).unwrap();
    let bob = ledger.create_account(AccountKind::Private, 50).unwrap();
    let store = Rc::new(RefCell::new(PermissionStore::new()));
    let gate = StoreGate::new(store.clone());
    let runtime = WalletRuntime::new(
        ledger,
        store,
        Box::new(gate),
        Box::new(ExplicitApprover::new()),
        Box::new(MockSigner::new()),
    );
    for account in [alice, bob] {
        for capability in [
            Capability::ProposeTransfer,
            Capability::SignTransaction,
            Capability::BalanceRead,
        ] {
            runtime
                .grant(capability, Scope::Account(account))
                .expect("dapp issued by this store");
        }
    }
    runtime
}

#[test]
fn public_send_is_inert_until_exact_approval() {
    let mut runtime = demo_runtime();
    let from = AccountId::new(1);
    let to = AccountId::new(2);
    let outcome = runtime.send(from, to, 30, Asset::Native).unwrap();
    assert_eq!(runtime.sign_count(), 0);
    assert_eq!(runtime.submission_count(), 0);
    assert_eq!(runtime.balance(from), Ok(1000));

    let wrong = ProposalBinding {
        amount: 31,
        ..outcome.binding.clone()
    };
    assert_eq!(
        runtime.approve(outcome.request_id, &wrong, from),
        Err(WalletError::WrongRequestFields)
    );
    assert_eq!(
        runtime.approve(outcome.request_id, &outcome.binding, to),
        Err(WalletError::WrongApprover {
            expected: from,
            found: to
        })
    );
    assert_eq!(runtime.sign_count(), 0);

    let receipt = runtime
        .approve(outcome.request_id, &outcome.binding, from)
        .unwrap();
    assert_eq!(runtime.balance(from), Ok(970));
    assert_eq!(runtime.balance(to), Ok(1030));
    assert_eq!(
        runtime.state(outcome.request_id),
        Ok(&ApprovalState::Submitted {
            token: wallet_cli::ApprovalToken::new("approval-token-1"),
            hash: receipt.hash,
        })
    );
    assert!(runtime.history(AccountId::new(3)).unwrap().is_empty());
}

#[test]
fn private_send_keeps_balance_parity_and_replay_fails() {
    let mut runtime = bare_runtime();
    let private = AccountId::new(2);
    let public = AccountId::new(1);
    let outcome = runtime.send(private, public, 20, Asset::Native).unwrap();
    assert_eq!(runtime.balance(private), Ok(50));
    let receipt = runtime
        .approve(outcome.request_id, &outcome.binding, private)
        .unwrap();
    assert_eq!(runtime.balance(private), Ok(30));
    assert_eq!(runtime.balance(public), Ok(70));
    assert!(runtime
        .approve(outcome.request_id, &outcome.binding, private)
        .is_err());
    assert_eq!(runtime.submission_count(), 1);
    assert_eq!(
        runtime
            .last_signature()
            .map(|signature| signature.as_bytes().len()),
        Some(32)
    );
    assert_eq!(receipt.hash.as_hex().len(), 64);
}

#[test]
fn restart_reconstructs_balances_and_clears_pending_state() {
    let mut runtime = demo_runtime();
    let from = AccountId::new(1);
    let to = AccountId::new(2);
    let first = runtime.send(from, to, 5, Asset::Native).unwrap();
    runtime
        .approve(first.request_id, &first.binding, from)
        .unwrap();
    let pending = runtime.send(from, to, 6, Asset::Native).unwrap();
    assert_eq!(runtime.pending_count(), 1);
    let events = runtime.restart().unwrap();
    assert_eq!(events, 6);
    assert_eq!(runtime.pending_count(), 0);
    assert!(runtime.state(pending.request_id).is_err());
    assert_eq!(runtime.balance(from), Ok(995));
    assert_eq!(runtime.balance(to), Ok(1005));
}

#[test]
fn deny_by_default_prevents_any_proposal() {
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
        Err(WalletError::Consent(_))
    ));
    assert_eq!(runtime.pending_count(), 0);
    assert_eq!(runtime.sign_count(), 0);
}
