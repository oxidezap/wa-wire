//! The gate, as a command.
//!
//! Deliberately thin: reading arguments, reading two files, printing, and
//! choosing an exit code. Every decision worth testing lives in the library
//! beside it, which is why this can be short enough to read in one go.

use std::process::ExitCode;

use wa_wire_gate::{Cli, USAGE, UsageError, exit, run};

fn main() -> ExitCode {
    let cli = match Cli::parse(std::env::args().skip(1)) {
        Ok(cli) => cli,
        Err(UsageError::HelpRequested) => {
            print!("{USAGE}");
            return code(exit::PASS);
        }
        Err(error) => {
            eprintln!("{error}\n\n{USAGE}");
            return code(exit::USAGE);
        }
    };

    // Destructured from a fixed-size array rather than collected: there are
    // exactly two paths, and a `Vec` would need an arm for a case that cannot
    // happen.
    let [baseline, candidate] = [&cli.baseline, &cli.candidate].map(std::fs::read);
    let (baseline, candidate) = match (baseline, candidate) {
        (Ok(baseline), Ok(candidate)) => (baseline, candidate),
        (Err(error), _) => {
            eprintln!("{}: {error}", cli.baseline);
            return code(exit::INPUT);
        }
        (_, Err(error)) => {
            eprintln!("{}: {error}", cli.candidate);
            return code(exit::INPUT);
        }
    };

    let outcome = run(&baseline, &candidate, cli.profile, cli.max_findings);
    print!("{}", outcome.report);
    code(outcome.exit_code())
}

/// `ExitCode` only carries a `u8`, and every code this tool uses fits.
fn code(value: i32) -> ExitCode {
    ExitCode::from(u8::try_from(value).unwrap_or(1))
}
