use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

pub(crate) fn run(input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_diff-pretty"))
        .arg("--no-pager")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn().unwrap();
    if let Err(error) = child.stdin.take().unwrap().write_all(input.as_bytes()) {
        let _ = child.kill();
        let _ = child.wait();
        panic!("failed to write diff-pretty input: {error}");
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().unwrap(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("diff-pretty did not finish successfully within the deadline");
            }
        }
    }
}
