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
