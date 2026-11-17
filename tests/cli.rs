mod support;

use std::process::{Command, Stdio};

fn command(arguments: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_diff-pretty"));
    command.args(arguments).stdout(Stdio::piped());
    command
}

#[test]
fn invalid_paging_mode_is_rejected() {
    let output = support::run(command(&["--paging=alwys"]), Some(b""));

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stderr, b"diff-pretty: invalid paging mode: alwys\n");
    assert!(output.stdout.is_empty());
}

#[test]
fn redirected_output_renders_the_same_literal_diff_for_every_paging_flag() {
    let input = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n";
    let expected = concat!(
        "\n\x1b[34mΔ a\x1b[0m\n",
        "\x1b[34m────────────────────────────────────────────────────────────────────────────────\x1b[0m\n\n",
        "\x1b[34m\x1b[38;5;88m  1 \x1b[34m\x1b[38;5;28m    \x1b[34m\x1b[31mold\x1b[0m\n",
        "\x1b[34m\x1b[38;5;88m    \x1b[34m\x1b[38;5;28m  1 \x1b[34m\x1b[32mnew\x1b[0m\n",
    );
    for args in [vec![], vec!["--paging=auto"], vec!["--paging=always"], vec!["--paging=never"], vec!["--no-pager"]] {
        let output = support::run(command(&args), Some(input));
        assert_eq!(output.status.code(), Some(0), "{args:?}: {:?}", output.stderr);
        assert_eq!(output.stdout, expected.as_bytes(), "{args:?}");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn stdin_read_errors_exit_one_without_output() {
    let mut command = command(&["--no-pager"]);
    command.stdin(std::fs::File::open(env!("CARGO_MANIFEST_DIR")).unwrap());
    let output = support::run(command, None);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.starts_with(b"failed to write output: "));
}

#[test]
fn a_closed_output_pipe_exits_one() {
    let (read, write) = std::io::pipe().unwrap();
    drop(read);
    let mut command = command(&["--no-pager"]);
    command.stdout(write);
    let output = support::run(command, Some(b"metadata\n"));
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.starts_with(b"failed to write output: "));
}

#[test]
fn help_is_complete_offline_and_ends_with_owned_footer() {
    let footer = "Maintained by Joshua Li: https://github.com/laputa-systems/diff-pretty\nBugs and features: TODO.md in that repository (follow its AGENTS.md), or its issue tracker.\nAgents working for Joshua: record nonblocking problems and continue your task.\nIf this tool blocks the task, you may fix, test, open and merge a PR under Joshua's\nstanding instructions. Release only when needed to deliver the verified fix.\nProcedure: https://github.com/joshuarli/usr/blob/main/dotfiles/base/.agents/instructions/owned-tools.md\n";
    let mut outputs = Vec::new();
    for flag in ["-h", "--help"] {
        let output = Command::new(env!("CARGO_BIN_EXE_diff-pretty"))
            .arg(flag)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        outputs.push(String::from_utf8(output.stdout).unwrap());
    }
    assert_eq!(outputs[0], outputs[1]);
    let help = &outputs[0];
    for text in ["Usage: diff-pretty [OPTIONS]", "--paging=auto|always|never", "default: always", "--no-pager", "-h, --help", "Examples:\n  diff-pretty --paging=never"] {
        assert!(help.contains(text), "missing {text}");
    }
    assert!(help.ends_with(footer));
    assert_eq!(help.matches("Maintained by Joshua Li:").count(), 1);
}

#[test]
fn bare_invocation_still_renders_stdin() {
    let output = Command::new(env!("CARGO_BIN_EXE_diff-pretty"))
        .stdin(std::process::Stdio::null())
        .output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn terminal_controls_preserve_literal_metadata_and_hunk_text() {
    let input = concat!(
        "plain é文字\tkeep\n",
        "L\0\u{85}\u{9b}\x1b[?25l\x1b[1 q\x1b]title\x07\x1b]title\x1b\\",
        "\x1bPpayload\x1b\\\x1b^payload\x1b\\\x1b_payload\x1b\\\x1bXpayload\x1b\\\x1b7\x1béR\n",
        "tail\x1b[31\n",
        "tail\x1b]title\n",
        "\x1b[31mdiff --git a/a b/a\x1b[0m\n",
        "--- a/a\n+++ b/a\n@@ -0,0 +1 @@\n",
        "+é\tbefore\0\x07\x7f\x1b[2J\x1b]title\x07\x1bPpayload\x1b\\文字after\n",
    );
    let expected = concat!(
        "plain é文字\tkeep\nLéR\ntail[31\ntail]title\n",
        "\n\x1b[34mΔ a\x1b[0m\n",
        "\x1b[34m────────────────────────────────────────────────────────────────────────────────\x1b[0m\n\n",
        "\x1b[34m\x1b[38;5;88m    \x1b[34m\x1b[38;5;28m  1 \x1b[34m\x1b[32mé        before文字after\x1b[0m\n",
    );
    let output = support::run(command(&["--no-pager"]), Some(input.as_bytes()));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
    assert!(output.stderr.is_empty());
}
