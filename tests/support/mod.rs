use std::io::{self, Write};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct OwnedChild(Option<Child>);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn run(mut command: Command, input: Option<&[u8]>) -> Output {
    command.stderr(Stdio::piped());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = OwnedChild(Some(command.spawn().expect("spawn command under test")));
    if let Some(input) = input {
        assert!(input.len() < 4096, "process fixture fits a pipe without a reader");
        let result = child.0.as_mut().unwrap().stdin.take().unwrap().write_all(input);
        if let Err(error) = result {
            assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        }
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.0.as_mut().unwrap().try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline, "command exceeded its five-second deadline");
        thread::sleep(Duration::from_millis(5));
    }
    child.0.take().unwrap().wait_with_output().unwrap()
}
