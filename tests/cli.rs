//! End-to-end checks of the `ti` binary's command line.
//!
//! Every case here exits before the interactive screen opens: when the tests
//! run inside a real terminal, the screen would take it over and wait for
//! keys. The interactive behaviour is unit-tested in `src/app.rs`.

use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

fn ti(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ti"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("failed to run ti")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Asserts `ti args` exits with `code` and `stderr` contains `needle`.
fn assert_fails(args: &[&str], code: i32, needle: &str) {
    let out = ti(args);
    assert_eq!(
        out.status.code(),
        Some(code),
        "ti {args:?}: {}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains(needle),
        "ti {args:?}: expected {needle:?} in {:?}",
        stderr(&out)
    );
    assert!(
        !stderr(&out).contains("panicked"),
        "ti {args:?} panicked: {}",
        stderr(&out)
    );
}

fn temp_file(name: &str, content: &str) -> PathBuf {
    let path = env::temp_dir().join(format!("ti_cli_test_{}_{name}", std::process::id()));
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn version_matches_cargo() {
    let out = ti(&["--version"]);
    assert!(out.status.success());
    assert_eq!(
        stdout(&out).trim(),
        format!("ti {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_lists_options_and_pomodoro() {
    let out = ti(&["--help"]);
    assert!(out.status.success());
    let help = stdout(&out);
    for needle in [
        "[TIME]",
        "--hours",
        "--minutes",
        "--seconds",
        "--font",
        "--cache",
        "--clear",
        "pomodoro",
    ] {
        assert!(help.contains(needle), "missing {needle:?} in:\n{help}");
    }
    assert!(!help.contains(r#"[default: ""]"#), "{help}");
}

#[test]
fn pomodoro_help_shows_classic_defaults() {
    let out = ti(&["pomodoro", "--help"]);
    assert!(out.status.success());
    let help = stdout(&out);
    for needle in [
        "--work <MINUTES>",
        "[default: 25]",
        "--break <MINUTES>",
        "[default: 5]",
        "--long-break <MINUTES>",
        "[default: 15]",
        "--rounds <ROUNDS>",
        "[default: 4]",
        "--font",
        "--cache",
        "--clear",
    ] {
        assert!(help.contains(needle), "missing {needle:?} in:\n{help}");
    }
}

#[test]
fn invalid_time_is_usage_error() {
    for args in [
        &["abc"][..],
        &["1:2:3:4"][..],
        &["1::2"][..],
        &["-1"][..],
        &["18446744073709551616"][..],
        &["18446744073709551615"][..],
        &["-M", "x"][..],
    ] {
        assert_fails(args, 2, "error:");
    }
}

#[test]
fn total_too_large_is_usage_error() {
    assert_fails(
        &["-H", "18446744073709551615", "-M", "1"],
        2,
        "the total time is too large",
    );
}

#[test]
fn pomodoro_invalid_lengths_are_usage_errors() {
    for flag in ["--work", "--break", "--long-break", "--rounds"] {
        assert_fails(&["pomodoro", flag, "0"], 2, flag);
        assert_fails(&["pomodoro", flag, "x"], 2, flag);
    }
    assert_fails(&["pomodoro", "--work", "307445734561825861"], 2, "--work");
}

#[test]
fn pomodoro_rejects_countdown_options() {
    assert_fails(&["-M", "5", "pomodoro"], 2, "--minutes");
    assert_fails(&["-H", "1", "pomodoro"], 2, "--hours");
    assert_fails(&["-S", "1", "pomodoro"], 2, "--seconds");
    assert_fails(&["1:30", "pomodoro"], 2, "[TIME]");
    assert_fails(&["pomodoro", "5"], 2, "unexpected argument");
    assert_fails(&["pomodoro", "-M", "5"], 2, "unexpected argument");
}

#[test]
fn unknown_subcommand_or_flag_is_usage_error() {
    assert_fails(&["--nope"], 2, "--nope");
    assert_fails(&["pomodoro", "--nope"], 2, "--nope");
}

#[test]
fn missing_font_is_error_in_both_modes() {
    let path = "/definitely/not/a/font.flf";
    for args in [
        &["--font", path, "1"][..],
        &["pomodoro", "--font", path][..],
        &["--font", path, "pomodoro"][..],
    ] {
        assert_fails(args, 1, "cannot read font");
    }
}

#[test]
fn invalid_fonts_are_errors() {
    let cases = [
        ("garbage", "this is not a figlet font", "invalid font"),
        (
            "zero_height",
            "flf2a$ 0 0 4 0 0\nx@\n",
            "malformed font file",
        ),
        (
            "no_digits",
            "flf2a$ 1 1 4 0 1\ncomment\n$@@\n",
            "no glyph for '0'",
        ),
    ];
    for (name, content, needle) in cases {
        let path = temp_file(name, content);
        let path_str = path.to_string_lossy().into_owned();
        assert_fails(&["--font", &path_str, "1"], 1, needle);
        assert_fails(&["pomodoro", "--font", &path_str], 1, needle);
        let _ = fs::remove_file(&path);
    }
}

#[test]
fn errors_do_not_touch_the_cache() {
    let cache = env::temp_dir().join(format!("ti_cli_test_{}_cache", std::process::id()));
    let _ = fs::remove_file(&cache);
    let cache_str = cache.to_string_lossy().into_owned();
    assert_fails(
        &["--cache", &cache_str, "--font", "/nope", "pomodoro"],
        1,
        "cannot read font",
    );
    assert_fails(&["--cache", &cache_str, "abc"], 2, "error:");
    assert!(!cache.exists());
}
