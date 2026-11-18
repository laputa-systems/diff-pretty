#[path = "../../tests/support/mod.rs"]
mod support;

use std::process::{Command, Stdio};

fn command(arguments: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scrl"));
    command.args(arguments).stdout(Stdio::piped());
    command
}

#[test]
fn redirected_input_is_byte_exact_for_all_noninteractive_options() {
    let input = "\x1b[31mred\x1b[0m\nλ\tlast".as_bytes();
    for args in [vec![], vec!["--paging=auto"], vec!["--paging=always"], vec!["--paging=never"],
        vec!["--no-pager", "--wrap", "--follow", "--filter=red"]] {
        let output = support::run(command(&args), Some(input));
        assert_eq!(output.status.code(), Some(0), "{args:?}: {:?}", output.stderr);
        assert_eq!(output.stdout, input, "{args:?}");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn help_returns_the_literal_usage_without_reading_stdin() {
    for flag in ["--help", "-h"] {
        let output = support::run(command(&[flag]), Some(b"\xff"));
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, b"usage: scrl [--paging=auto|always|never] [--no-pager] [--wrap] [--follow] [--filter=REGEX] [FILE ...]\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn malformed_options_exit_two_and_invalid_utf8_exits_one() {
    let output = support::run(command(&["--paging=alwys"]), Some(b""));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stderr, b"scrl: invalid paging mode: alwys\n");
    assert!(output.stdout.is_empty());

    let output = support::run(command(&["--no-pager"]), Some(b"\xff"));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stderr, b"scrl: invalid utf-8 sequence of 1 bytes from index 0\n");
    assert!(output.stdout.is_empty());

    let output = support::run(command(&["--filter=["]), Some(b""));
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.starts_with(b"scrl: "));
    assert!(output.stderr.windows(b"unclosed character class".len())
        .any(|part| part == b"unclosed character class"));
    assert!(output.stdout.is_empty());
}

#[test]
fn file_operands_concatenate_without_a_separator_and_missing_input_fails() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures");
    let first = root.join("cli-first.txt");
    let second = root.join("cli-second.txt");
    for operands in [vec![&first], vec![&first, &second]] {
        let mut command = command(&["--no-pager"]);
        command.args(&operands);
        let output = support::run(command, Some(b"ignored stdin\n"));
        assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
        let expected = if operands.len() == 1 { &b"first\n"[..] } else { &b"first\nsecond\n"[..] };
        assert_eq!(output.stdout, expected);
        assert!(output.stderr.is_empty());
    }
    let mut command = command(&["--no-pager"]);
    command.arg(root.join("not-present"));
    let output = support::run(command, Some(b""));
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.starts_with(b"scrl: "));
    assert!(output.stdout.is_empty());
}

#[test]
fn a_closed_output_pipe_exits_one() {
    let (read, write) = std::io::pipe().unwrap();
    drop(read);
    let mut command = command(&["--no-pager"]);
    command.stdout(write);
    let output = support::run(command, Some(b"plain text\n"));
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.starts_with(b"scrl: "));
}
