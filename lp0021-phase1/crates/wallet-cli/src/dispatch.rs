// SPDX-License-Identifier: MIT AND Apache-2.0
//! Command dispatcher. Every command funnels through [`WalletRuntime`]; no
//! command can reach the signer without the explicit `approve` path.

use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

use crate::args::{usage, Command};
use crate::runtime::{WalletError, WalletRuntime};

/// Dispatch failure.
#[derive(Debug)]
pub enum CliError {
    /// Runtime refused the operation.
    Wallet(WalletError),
    /// Output stream failure.
    Io(io::Error),
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wallet(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "I/O error: {error}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<WalletError> for CliError {
    fn from(error: WalletError) -> Self {
        Self::Wallet(error)
    }
}

impl From<io::Error> for CliError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Execute one parsed command against a runtime and write human-readable
/// output to `out`.
pub fn dispatch(
    runtime: &mut WalletRuntime,
    command: Command,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    match command {
        Command::Balance { account } => {
            let amount = runtime.balance(account)?;
            writeln!(out, "balance {account} native {amount}")?;
        }
        Command::Send {
            from,
            to,
            amount,
            asset,
        } => {
            let outcome = runtime.send(from, to, amount, asset)?;
            writeln!(
                out,
                "proposal {} AwaitingApproval (no signature, no submission)",
                outcome.request_id
            )?;
            for line in outcome.effects.human_lines() {
                writeln!(out, "  {line}")?;
            }
            writeln!(
                out,
                "  approve with: approve {} --from {from} --to {to} --amount {amount} \
                 --nonce {} --account {from}",
                outcome.request_id, outcome.binding.nonce
            )?;
        }
        Command::Receive { kind } => {
            let handle = runtime.receive(kind)?;
            writeln!(out, "receive {handle}")?;
        }
        Command::History { account } => {
            for entry in runtime.history(account)? {
                let peer = if entry.direction == wallet_chain::Direction::Sent {
                    entry.to
                } else {
                    entry.from
                };
                writeln!(
                    out,
                    "history {account} {} {} {} {} {} nonce={} peer={peer}",
                    entry.hash,
                    entry.direction,
                    entry.amount,
                    entry.asset,
                    entry.shape,
                    entry.nonce
                )?;
            }
        }
        Command::Approve {
            request_id,
            binding,
            approver,
        } => {
            let receipt = runtime.approve(request_id, &binding, approver)?;
            writeln!(out, "approved {request_id} submitted {}", receipt.hash)?;
        }
        Command::Reject { request_id, reason } => {
            runtime.reject(request_id, reason)?;
            writeln!(out, "rejected {request_id}")?;
        }
        Command::Restart => {
            let events = runtime.restart()?;
            writeln!(
                out,
                "restart replayed {events} events; pending proposals cleared"
            )?;
        }
        Command::Help => {
            writeln!(out, "{}", usage())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::parse_command;
    use crate::runtime::demo_runtime;
    use wallet_chain::AccountId;

    fn argv(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn run(runtime: &mut WalletRuntime, args: &[&str]) -> Result<String, CliError> {
        let command = parse_command(&argv(args)).expect("valid test command");
        let mut out = Vec::new();
        dispatch(runtime, command, &mut out)?;
        Ok(String::from_utf8(out).expect("test output is UTF-8"))
    }

    #[test]
    fn dispatcher_runs_balance_send_approve_history_restart() {
        let mut runtime = demo_runtime();
        let balance = run(&mut runtime, &["balance", "--account", "acct-1"]).unwrap();
        assert!(balance.contains("balance acct-1 native 1000"));
        let send = run(
            &mut runtime,
            &[
                "send", "--from", "acct-1", "--to", "acct-2", "--amount", "25",
            ],
        )
        .unwrap();
        assert!(send.contains("proposal req-0001 AwaitingApproval"));
        assert!(send.contains("LOCAL SIMULATION ONLY"));
        let approve = run(
            &mut runtime,
            &[
                "approve",
                "req-0001",
                "--from",
                "acct-1",
                "--to",
                "acct-2",
                "--amount",
                "25",
                "--nonce",
                "0",
                "--account",
                "acct-1",
            ],
        )
        .unwrap();
        assert!(approve.contains("approved req-0001 submitted"));
        let history = run(&mut runtime, &["history", "--account", "acct-1"]).unwrap();
        assert!(history.contains("history acct-1"));
        assert!(history.contains("peer=acct-2"));
        let restart = run(&mut runtime, &["restart"]).unwrap();
        assert!(restart.contains("pending proposals cleared"));
        let after = run(&mut runtime, &["balance", "--account", "acct-1"]).unwrap();
        assert!(after.contains("native 975"));
    }

    #[test]
    fn dispatcher_reports_approval_gate_errors() {
        let mut runtime = demo_runtime();
        let error = run(
            &mut runtime,
            &[
                "approve",
                "req-0001",
                "--from",
                "acct-1",
                "--to",
                "acct-2",
                "--amount",
                "1",
                "--nonce",
                "0",
                "--account",
                "acct-1",
            ],
        )
        .unwrap_err();
        assert!(error.to_string().contains("unknown proposal"));
        assert_eq!(runtime.submission_count(), 0);
    }

    #[test]
    fn dispatcher_receive_and_reject_commands() {
        let mut runtime = demo_runtime();
        let receive = run(&mut runtime, &["receive", "--kind", "private"]).unwrap();
        assert!(receive.starts_with("receive recv-private-"));
        let _ = run(
            &mut runtime,
            &[
                "send", "--from", "acct-1", "--to", "acct-3", "--amount", "3",
            ],
        )
        .unwrap();
        let rejected = run(&mut runtime, &["reject", "req-0001"]).unwrap();
        assert_eq!(rejected.trim(), "rejected req-0001");
        assert_eq!(runtime.balance(AccountId::new(1)), Ok(1000));
    }
}
