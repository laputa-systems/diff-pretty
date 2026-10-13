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
