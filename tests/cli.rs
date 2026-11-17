use std::process::Command;

#[test]
fn invalid_paging_mode_is_rejected() {
    let output = Command::new(env!("CARGO_BIN_EXE_diff-pretty"))
        .arg("--paging=alwys")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid paging mode"));
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

#[path = "support/bare_controls.rs"]
mod bare_controls;

#[test]
fn terminal_controls_preserve_metadata_and_hunk_text() {
    let cases = [
        (
            "--- a/same\n+++ b/same\n@@ -0,0 +1 @@\n+é\0\x7f\u{85}\u{9b}x\tend\n",
            concat!("\n\x1b[34mΔ a/same ⟶   b/same\x1b[0m\n", "\x1b[34m────────────────────────────────────────────────────────────────────────────────\x1b[0m\n\n", "\x1b[34m\x1b[38;5;88m    \x1b[34m\x1b[38;5;28m  1 \x1b[34m\x1b[32méx        end\x1b[0m\n"),
        ),
        (
            concat!("plain é文字\tkeep\n", "\x1b[31mdiff --git a/a b/a\x1b[0m\n", "--- a/a\n+++ b/a\n@@ -0,0 +1 @@\n", "+é\tbefore\0\x07\x7f\x1b[2J\x1b]title\x07\x1bPpayload\x1b\\文字after\n"),
            concat!("plain é文字\tkeep\n", "\n\x1b[34mΔ a\x1b[0m\n", "\x1b[34m────────────────────────────────────────────────────────────────────────────────\x1b[0m\n\n", "\x1b[34m\x1b[38;5;88m    \x1b[34m\x1b[38;5;28m  1 \x1b[34m\x1b[32mé        before文字after\x1b[0m\n"),
        ),
    ];
    for (input, expected) in cases {
        let output = bare_controls::run(input);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, expected.as_bytes());
        assert!(output.stderr.is_empty());
    }
}
