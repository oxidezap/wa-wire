//! The binary, run the way a CI step runs it.
//!
//! The library tests decide what the gate concludes; these check the part that
//! only exists once it is a command — that the exit codes a pipeline branches
//! on are the ones the usage text promises, and that a missing file is told
//! apart from a bad verdict.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use wa_wire_l1::testing::Fixture;
use wa_wire_recording::{ArtifactClass, MetaBuilder, RecordingWriter};

/// Cargo hands integration tests a directory of their own.
fn tmp() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

fn write_recording(name: &str, ids: &[&str], input: &[u8]) -> PathBuf {
    let meta = MetaBuilder::new()
        .adapter("engine", "0.1.0", "1.0", 1, ["l0.inbound.tap"])
        .expect("adapter")
        .artifact_class(ArtifactClass::Replayed)
        .expect("class")
        .input_digest(input)
        .expect("input");

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

fn gate<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_wa-wire-gate"))
        .args(args)
        .output()
        .expect("the gate runs")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("the gate exits normally")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn arg(path: &Path) -> String {
    path.display().to_string()
}

#[test]
fn a_pass_exits_zero_and_says_so() {
    let one = write_recording("pass-a.wawr", &["A", "B"], b"monday");
    let output = gate([&arg(&one), &arg(&one)]);

    assert_eq!(code(&output), 0);
    assert!(stdout(&output).contains("PASS"), "{}", stdout(&output));
    assert!(stderr(&output).is_empty(), "nothing to complain about");
}

#[test]
fn a_regression_exits_one() {
    let baseline = write_recording("fail-baseline.wawr", &["A", "B"], b"monday");
    let candidate = write_recording("fail-candidate.wawr", &["A"], b"monday");

    let output = gate([&arg(&baseline), &arg(&candidate)]);
    assert_eq!(code(&output), 1);
    assert!(stdout(&output).contains("FAIL under regression"));
}

#[test]
fn an_incomparable_pair_exits_two_rather_than_one() {
    // The distinction the exit codes exist for: a pipeline that treated this
    // as a failure would send someone looking for a bug that is not there,
    // and one that treated it as a pass would ship on no evidence at all.
    let monday = write_recording("incomparable-a.wawr", &["A"], b"monday");
    let tuesday = write_recording("incomparable-b.wawr", &["A"], b"tuesday");

    let output = gate([&arg(&monday), &arg(&tuesday)]);
    assert_eq!(code(&output), 2);
    assert!(stdout(&output).contains("INCOMPARABLE"));
    assert!(stdout(&output).contains("different input traffic"));
}

#[test]
fn the_profile_is_selectable_from_the_command_line() {
    let baseline = write_recording("profile-a.wawr", &["A"], b"monday");
    let candidate = write_recording("profile-b.wawr", &["B"], b"monday");

    let regression = gate([&arg(&baseline), &arg(&candidate)]);
    assert_eq!(code(&regression), 1, "differing bytes fail a regression");

    let interop = gate([
        "--profile".to_owned(),
        "interop".to_owned(),
        arg(&baseline),
        arg(&candidate),
    ]);
    assert_eq!(
        code(&interop),
        1,
        "and the derived events differ too, so interop fails as well"
    );
    assert!(stdout(&interop).contains("under interop"));
}

#[test]
fn a_missing_file_is_told_apart_from_a_bad_verdict() {
    let one = write_recording("missing-a.wawr", &["A"], b"monday");
    let output = gate([&arg(&one), &arg(&tmp().join("does-not-exist.wawr"))]);

    assert_eq!(code(&output), 66);
    assert!(
        stderr(&output).contains("does-not-exist.wawr"),
        "it has to name the file: {}",
        stderr(&output)
    );
}

#[test]
fn a_missing_baseline_names_the_baseline() {
    // Both sides are reported by name, because "file not found" without one
    // sends a reader to check the wrong path.
    let one = write_recording("missing-b.wawr", &["A"], b"monday");
    let output = gate([&arg(&tmp().join("no-baseline.wawr")), &arg(&one)]);

    assert_eq!(code(&output), 66);
    assert!(
        stderr(&output).contains("no-baseline.wawr"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_file_that_is_not_a_recording_exits_as_unreadable_input() {
    let good = write_recording("garbage-a.wawr", &["A"], b"monday");
    let garbage = tmp().join("garbage.wawr");
    std::fs::write(&garbage, b"not a recording").expect("write");

    let output = gate([&arg(&good), &arg(&garbage)]);
    assert_eq!(code(&output), 66);
    assert!(stdout(&output).contains("candidate"), "{}", stdout(&output));
}

#[test]
fn bad_arguments_exit_sixty_four_and_print_the_usage() {
    for args in [
        vec!["--profile", "sideways", "a", "b"],
        vec!["only-one-path"],
        vec!["--nonsense"],
        vec!["a", "b", "c"],
    ] {
        let output = gate(args.clone());
        assert_eq!(code(&output), 64, "{args:?}");
        assert!(
            stderr(&output).contains("wa-wire-gate"),
            "{args:?} must print the usage"
        );
    }
}

#[test]
fn help_prints_to_stdout_and_exits_zero() {
    // Asked for, so it is not an error: a pipeline running `--help` in a
    // smoke check should not see a failure.
    for flag in ["-h", "--help"] {
        let output = gate([flag]);
        assert_eq!(code(&output), 0, "{flag}");
        assert!(stdout(&output).contains("wa-wire-gate"), "{flag}");
        assert!(stderr(&output).is_empty(), "{flag}");
    }
}
