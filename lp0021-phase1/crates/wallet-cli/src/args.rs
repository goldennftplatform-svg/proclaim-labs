// SPDX-License-Identifier: MIT AND Apache-2.0
//! Zero-dependency argument parser and command model for `wallet-cli`.
//!
//! Accepted forms:
//! ```text
//! wallet-cli balance --account acct-1
//! wallet-cli send --from acct-1 --to acct-2 --amount 100 [--asset native]
//! wallet-cli receive --kind public|private
//! wallet-cli history --account acct-1
//! wallet-cli approve req-0001 --from acct-1 --to acct-2 --amount 100 \
//!     --nonce 0 --account acct-1 [--asset native]
//! wallet-cli reject req-0001 [--reason "..."]
//! wallet-cli restart
//! ```
//! A single process can run a scripted sequence by separating commands with a
//! literal `--` token, e.g. `send ... -- approve ... -- restart`.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

use wallet_chain::ParseAccountIdError;
use wallet_chain::{AccountId, AccountKind, Asset};

use crate::approval::{ParseRequestIdError, ProposalBinding, RequestId};

const KNOWN_FLAGS: &[&str] = &[
    "account", "amount", "asset", "from", "kind", "nonce", "reason", "to",
];

/// Parsed CLI command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Show native balance of one account.
    Balance { account: AccountId },
    /// Build an inert transfer proposal.
    Send {
        from: AccountId,
        to: AccountId,
        amount: u128,
        asset: Asset,
    },
    /// Generate an opaque receive handle.
    Receive { kind: AccountKind },
    /// Show one account's transfer history.
    History { account: AccountId },
    /// Approve an exact pending proposal; this is the only signing path.
    Approve {
        request_id: RequestId,
        binding: ProposalBinding,
        approver: AccountId,
    },
    /// Reject a pending proposal.
    Reject {
        request_id: RequestId,
        reason: String,
    },
    /// Rebuild the ledger from its event log and drop volatile proposals.
    Restart,
    /// Print usage.
    Help,
}

/// Argument parsing failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// No command was supplied.
    MissingCommand,
    /// The command name is not recognized.
    UnknownCommand(String),
    /// A flag has no value.
    MissingValue(&'static str),
    /// The flag is not known at all.
    UnknownFlag(String),
    /// A flag was supplied more than once.
    DuplicateFlag(&'static str),
    /// A flag is not valid for the chosen command.
    FlagNotAllowed { command: &'static str, flag: String },
    /// Unexpected positional argument.
    UnexpectedArgument(String),
    /// A flag value could not be parsed.
    InvalidValue { flag: &'static str, value: String },
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => write!(f, "missing command (try 'help')"),
            Self::UnknownCommand(command) => write!(f, "unknown command: {command}"),
            Self::MissingValue(flag) => write!(f, "flag --{flag} needs a value"),
            Self::UnknownFlag(flag) => write!(f, "unknown flag: --{flag}"),
            Self::DuplicateFlag(flag) => write!(f, "flag --{flag} was supplied twice"),
            Self::FlagNotAllowed { command, flag } => {
                write!(f, "flag --{flag} is not valid for {command}")
            }
            Self::UnexpectedArgument(value) => write!(f, "unexpected argument: {value}"),
            Self::InvalidValue { flag, value } => {
                write!(f, "invalid value for --{flag}: {value}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

fn bad(flag: &'static str, value: &str) -> ParseError {
    ParseError::InvalidValue {
        flag,
        value: value.to_owned(),
    }
}

fn account(
    flags: &BTreeMap<&'static str, String>,
    flag: &'static str,
) -> Result<AccountId, ParseError> {
    let value = flags.get(flag).ok_or(ParseError::MissingValue(flag))?;
    value
        .parse::<AccountId>()
        .map_err(|_: ParseAccountIdError| bad(flag, value))
}

fn number(flags: &BTreeMap<&'static str, String>, flag: &'static str) -> Result<u128, ParseError> {
    let value = flags.get(flag).ok_or(ParseError::MissingValue(flag))?;
    value.parse::<u128>().map_err(|_| bad(flag, value))
}

fn nonce(flags: &BTreeMap<&'static str, String>) -> Result<u64, ParseError> {
    let value = flags
        .get("nonce")
        .ok_or(ParseError::MissingValue("nonce"))?;
    value.parse::<u64>().map_err(|_| bad("nonce", value))
}

fn asset(flags: &BTreeMap<&'static str, String>) -> Result<Asset, ParseError> {
    match flags.get("asset") {
        None => Ok(Asset::Native),
        Some(value) => value.parse::<Asset>().map_err(|_| bad("asset", value)),
    }
}

fn kind(flags: &BTreeMap<&'static str, String>) -> Result<AccountKind, ParseError> {
    let value = flags.get("kind").ok_or(ParseError::MissingValue("kind"))?;
    value.parse::<AccountKind>().map_err(|_| bad("kind", value))
}

fn request_id(positionals: &[String]) -> Result<RequestId, ParseError> {
    let value = positionals
        .first()
        .ok_or_else(|| bad("request", "<missing>"))?;
    value
        .parse::<RequestId>()
        .map_err(|_: ParseRequestIdError| bad("request", value))
}

fn only(
    flags: &BTreeMap<&'static str, String>,
    command: &'static str,
    allowed: &[&str],
) -> Result<(), ParseError> {
    for flag in flags.keys() {
        if !allowed.contains(flag) {
            return Err(ParseError::FlagNotAllowed {
                command,
                flag: (*flag).to_owned(),
            });
        }
    }
    Ok(())
}

fn no_positionals(positionals: &[String]) -> Result<(), ParseError> {
    match positionals.first() {
        Some(value) => Err(ParseError::UnexpectedArgument(value.clone())),
        None => Ok(()),
    }
}

/// Parse one command from argv-style arguments (including the command name).
pub fn parse_command(args: &[String]) -> Result<Command, ParseError> {
    let name = args.first().ok_or(ParseError::MissingCommand)?.as_str();
    if matches!(name, "help" | "--help" | "-h") {
        return Ok(Command::Help);
    }
    let command: &'static str = match name {
        "balance" => "balance",
        "send" => "send",
        "receive" => "receive",
        "history" => "history",
        "approve" => "approve",
        "reject" => "reject",
        "restart" => "restart",
        other => return Err(ParseError::UnknownCommand(other.to_owned())),
    };

    let mut flags: BTreeMap<&'static str, String> = BTreeMap::new();
    let mut positionals: Vec<String> = Vec::new();
    let mut index = 1;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--help" || argument == "-h" {
            return Ok(Command::Help);
        }
        if let Some(stripped) = argument.strip_prefix("--") {
            let (raw_name, inline_value) = match stripped.split_once('=') {
                Some((flag, value)) => (flag, Some(value.to_owned())),
                None => (stripped, None),
            };
            let flag = *KNOWN_FLAGS
                .iter()
                .find(|candidate| **candidate == raw_name)
                .ok_or_else(|| ParseError::UnknownFlag(raw_name.to_owned()))?;
            if flags.contains_key(flag) {
                return Err(ParseError::DuplicateFlag(flag));
            }
            let value = match inline_value {
                Some(value) => value,
                None => {
                    index += 1;
                    args.get(index)
                        .ok_or(ParseError::MissingValue(flag))?
                        .clone()
                }
            };
            flags.insert(flag, value);
        } else {
            positionals.push(argument.clone());
        }
        index += 1;
    }

    match command {
        "balance" => {
            only(&flags, command, &["account"])?;
            no_positionals(&positionals)?;
            Ok(Command::Balance {
                account: account(&flags, "account")?,
            })
        }
        "send" => {
            only(&flags, command, &["from", "to", "amount", "asset"])?;
            no_positionals(&positionals)?;
            Ok(Command::Send {
                from: account(&flags, "from")?,
                to: account(&flags, "to")?,
                amount: number(&flags, "amount")?,
                asset: asset(&flags)?,
            })
        }
        "receive" => {
            only(&flags, command, &["kind"])?;
            no_positionals(&positionals)?;
            Ok(Command::Receive {
                kind: kind(&flags)?,
            })
        }
        "history" => {
            only(&flags, command, &["account"])?;
            no_positionals(&positionals)?;
            Ok(Command::History {
                account: account(&flags, "account")?,
            })
        }
        "approve" => {
            only(
                &flags,
                command,
                &["from", "to", "amount", "asset", "nonce", "account"],
            )?;
            if positionals.len() > 1 {
                return Err(ParseError::UnexpectedArgument(positionals[1].clone()));
            }
            let id = request_id(&positionals)?;
            Ok(Command::Approve {
                request_id: id,
                binding: ProposalBinding {
                    from: account(&flags, "from")?,
                    to: account(&flags, "to")?,
                    amount: number(&flags, "amount")?,
                    asset: asset(&flags)?,
                    nonce: nonce(&flags)?,
                },
                approver: account(&flags, "account")?,
            })
        }
        "reject" => {
            only(&flags, command, &["reason"])?;
            if positionals.len() > 1 {
                return Err(ParseError::UnexpectedArgument(positionals[1].clone()));
            }
            Ok(Command::Reject {
                request_id: request_id(&positionals)?,
                reason: flags
                    .get("reason")
                    .cloned()
                    .unwrap_or_else(|| "user rejected".to_owned()),
            })
        }
        "restart" => {
            only(&flags, command, &[])?;
            no_positionals(&positionals)?;
            Ok(Command::Restart)
        }
        _ => unreachable!("validated command name above"),
    }
}

/// Split a scripted argv into segments at literal `--` separators and parse
/// each segment.
pub fn parse_commands(args: &[String]) -> Result<Vec<Command>, ParseError> {
    let mut commands = Vec::new();
    let mut segment: Vec<String> = Vec::new();
    for argument in args {
        if argument == "--" {
            if !segment.is_empty() {
                commands.push(parse_command(&segment)?);
                segment.clear();
            }
        } else {
            segment.push(argument.clone());
        }
    }
    if !segment.is_empty() {
        commands.push(parse_command(&segment)?);
    }
    Ok(commands)
}

/// Usage text printed by `help`.
pub fn usage() -> &'static str {
    "commands: balance --account ID | send --from ID --to ID --amount N [--asset native] \
     | receive --kind public|private | history --account ID \
     | approve REQ --from ID --to ID --amount N --nonce N --account ID [--asset native] \
     | reject REQ [--reason TEXT] | restart | help"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn parses_all_required_commands() {
        assert_eq!(
            parse_command(&argv(&["balance", "--account", "acct-1"])),
            Ok(Command::Balance {
                account: AccountId::new(1)
            })
        );
        assert!(matches!(
            parse_command(&argv(&[
                "send", "--from", "acct-1", "--to", "acct-2", "--amount", "5"
            ])),
            Ok(Command::Send { amount: 5, .. })
        ));
        assert_eq!(
            parse_command(&argv(&["receive", "--kind", "private"])),
            Ok(Command::Receive {
                kind: AccountKind::Private
            })
        );
        assert!(matches!(
            parse_command(&argv(&["history", "--account", "acct-2"])),
            Ok(Command::History { .. })
        ));
        assert!(matches!(
            parse_command(&argv(&["restart"])),
            Ok(Command::Restart)
        ));
    }

    #[test]
    fn parses_approve_and_reject_with_bindings() {
        let approval = parse_command(&argv(&[
            "approve",
            "req-0001",
            "--from",
            "acct-1",
            "--to",
            "acct-2",
            "--amount",
            "5",
            "--nonce",
            "0",
            "--account",
            "acct-1",
        ]))
        .unwrap();
        match approval {
            Command::Approve {
                request_id,
                binding,
                approver,
            } => {
                assert_eq!(request_id, RequestId::new(1));
                assert_eq!(binding.amount, 5);
                assert_eq!(binding.nonce, 0);
                assert_eq!(approver, AccountId::new(1));
            }
            other => panic!("expected approve command, got {other:?}"),
        }
        assert!(matches!(
            parse_command(&argv(&["reject", "req-0002"])),
            Ok(Command::Reject { .. })
        ));
    }

    #[test]
    fn accepts_inline_flag_values_and_split_scripts() {
        assert!(matches!(
            parse_command(&argv(&["balance", "--account=acct-3"])),
            Ok(Command::Balance { .. })
        ));
        let commands =
            parse_commands(&argv(&["restart", "--", "receive", "--kind", "public"])).unwrap();
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn rejects_unknown_command_and_flag() {
        assert_eq!(
            parse_command(&argv(&["frobnicate"])),
            Err(ParseError::UnknownCommand("frobnicate".to_owned()))
        );
        assert_eq!(
            parse_command(&argv(&["balance", "--magic", "1"])),
            Err(ParseError::UnknownFlag("magic".to_owned()))
        );
    }

    #[test]
    fn rejects_missing_and_duplicate_values() {
        assert_eq!(
            parse_command(&argv(&["balance", "--account"])),
            Err(ParseError::MissingValue("account"))
        );
        assert_eq!(
            parse_command(&argv(&[
                "balance",
                "--account",
                "acct-1",
                "--account",
                "acct-2"
            ])),
            Err(ParseError::DuplicateFlag("account"))
        );
    }

    #[test]
    fn rejects_invalid_values_and_wrong_scope_flags() {
        assert!(matches!(
            parse_command(&argv(&["balance", "--account", "0xdeadbeef"])),
            Err(ParseError::InvalidValue { .. })
        ));
        assert!(matches!(
            parse_command(&argv(&[
                "send", "--from", "acct-1", "--to", "acct-2", "--amount", "abc"
            ])),
            Err(ParseError::InvalidValue { .. })
        ));
        assert!(matches!(
            parse_command(&argv(&["restart", "--account", "acct-1"])),
            Err(ParseError::FlagNotAllowed { .. })
        ));
    }
}
