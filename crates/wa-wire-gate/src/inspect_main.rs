//! The inspector, as a command.
//!
//! Thin for the same reason [`wa-wire-gate`](main) is: reading arguments,
//! reading one file, printing, and choosing an exit code. Everything worth
//! testing is in the library beside it.

use std::process::ExitCode;

use wa_wire_gate::{INSPECT_USAGE, InspectCli, UsageError, exit, inspect};

fn main() -> ExitCode {
    let cli = match InspectCli::parse(std::env::args().skip(1)) {
        Ok(cli) => cli,
        Err(UsageError::HelpRequested) => {
            print!("{INSPECT_USAGE}");
            return code(exit::PASS);
        }
        Err(error) => {
            eprintln!("{error}\n\n{INSPECT_USAGE}");
            return code(exit::USAGE);
        }
    };

    let bytes = match std::fs::read(&cli.path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{}: {error}", cli.path);
            return code(exit::INPUT);
        }
    };

    let report = inspect(&bytes, cli.detail);
    print!("{}", report.text);
    code(report.exit_code())
}

/// `ExitCode` only carries a `u8`, and every code this tool uses fits.
fn code(value: i32) -> ExitCode {
    ExitCode::from(u8::try_from(value).unwrap_or(1))
}
