// SPDX-License-Identifier: MIT AND Apache-2.0
//! Entry point for the LP-0021 Phase 1 local CLI.
//!
//! Each invocation is an isolated in-memory process. Use `--` between commands
//! to run a scripted sequence in one process, for example:
//! ```text
//! wallet-cli send --from acct-1 --to acct-2 --amount 10 -- \
//!   approve req-0001 --from acct-1 --to acct-2 --amount 10 --nonce 0 --account acct-1 -- \
//!   restart
//! ```

use std::io::{self, Write};

use wallet_cli::args::{parse_commands, usage};
use wallet_cli::dispatch::dispatch;
use wallet_cli::runtime::demo_runtime;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let commands = match parse_commands(&args) {
        Ok(commands) => commands,
        Err(error) => {
            eprintln!("argument error: {error}");
            eprintln!("{}", usage());
            std::process::exit(2);
        }
    };
    let commands = if commands.is_empty() {
        vec![wallet_cli::args::Command::Help]
    } else {
        commands
    };

    let mut runtime = demo_runtime();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for command in commands {
        if let Err(error) = dispatch(&mut runtime, command, &mut out) {
            let _ = out.flush();
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
