//! The inspector, run the way someone handed a file runs it.
//!
//! The library tests decide what it says; these check the part that only
//! exists once it is a command — that a file it cannot read is told apart from
//! one it can, and that the exit codes match what the usage text promises.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use wa_wire_l1::testing::Fixture;
use wa_wire_recording::{ArtifactClass, MetaBuilder, RecordingWriter};

/// Cargo hands integration tests a directory of their own.
fn tmp() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

fn write_recording(name: &str, ids: &[&str]) -> PathBuf {
    let meta = MetaBuilder::new()
        .adapter("engine", "0.1.0", "1.0", 1, ["l0.inbound.tap"])
        .expect("adapter")
        .artifact_class(ArtifactClass::Replayed)
        .expect("class");

    let mut writer = RecordingWriter::new(meta).expect("writer");
    for id in ids {
        let fixture = Fixture::node("receipt")
            .attr("id", id)
            .jid_attr("from", "5511999998888")
            .attr("type", "read")
            .build();
        let envelope = wa_wire_adapter::RawStanza::inbound(fixture.bytes())
            .encode_to_vec()
            .expect("envelope");
        writer.envelope(&envelope).expect("write");
    }

    let path = tmp().join(name);
    std::fs::create_dir_all(tmp()).expect("tmp dir");
    std::fs::write(&path, writer.finish()).expect("write recording");
    path
}

fn inspect<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_wa-wire-inspect"))
        .args(args)
        .output()
        .expect("the inspector runs")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("it exits normally")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn arg(path: &Path) -> String {
    path.display().to_string()
}

#[test]
fn a_recording_is_described_and_exits_zero() {
    let path = write_recording("inspect-ok.wawr", &["A", "B"]);
    let output = inspect([arg(&path)]);

    assert_eq!(code(&output), 0);
    let text = stdout(&output);
    assert!(text.contains("engine 0.1.0"), "{text}");
    assert!(text.contains("2 · 2 inbound, 0 outbound"), "{text}");
}

#[test]
fn envelopes_are_listed_only_when_asked() {
    let path = write_recording("inspect-list.wawr", &["A"]);

    assert!(!stdout(&inspect([arg(&path)])).contains("#0"));
    assert!(stdout(&inspect(["--envelopes", &arg(&path)])).contains("<receipt id=A>"));
}

#[test]
fn a_file_that_is_not_a_recording_exits_sixty_six() {
    let path = tmp().join("inspect-garbage.wawr");
    std::fs::create_dir_all(tmp()).expect("tmp dir");
    std::fs::write(&path, b"not a recording").expect("write");

    let output = inspect([arg(&path)]);
    assert_eq!(code(&output), 66, "{}", stdout(&output));
}

#[test]
fn a_missing_file_exits_sixty_six_too() {
    // Same code as an unreadable one: from a caller's side both mean "there was
    // nothing here to describe", and splitting them would ask a pipeline to
    // branch on a difference it cannot act on.
    let output = inspect([arg(&tmp().join("does-not-exist.wawr"))]);
    assert_eq!(code(&output), 66);
}

#[test]
fn bad_arguments_exit_sixty_four_and_print_the_usage() {
    let output = inspect(["--nope"]);
    assert_eq!(code(&output), 64);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("wa-wire-inspect"),
        "the usage goes to stderr"
    );
}

#[test]
fn help_exits_zero() {
    let output = inspect(["--help"]);
    assert_eq!(code(&output), 0);
    assert!(stdout(&output).contains("say what is inside one recording"));
}
