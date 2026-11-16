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
