use std::io::{Read, Write};
use std::os::fd::BorrowedFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rustix::fs::{Mode, OFlags};
use rustix::pty::OpenptFlags;
use rustix::termios::{self, LocalModes, OptionalActions, OutputModes, Winsize};

#[test]
fn short_auto_output_keeps_post_processing() {
    let master = rustix::pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)
        .expect("open pty master");
    rustix::pty::grantpt(&master).expect("grant pty");
    rustix::pty::unlockpt(&master).expect("unlock pty");
    let name = rustix::pty::ptsname(&master, Vec::new()).expect("pty name");
    let slave = rustix::fs::open(name.as_c_str(), OFlags::RDWR | OFlags::NOCTTY, Mode::empty())
        .expect("open pty slave");
    let mut cooked = termios::tcgetattr(&slave).expect("slave termios");
    cooked.output_modes = OutputModes::OPOST | OutputModes::ONLCR;
    cooked
        .local_modes
        .insert(LocalModes::ISIG | LocalModes::ICANON | LocalModes::ECHO);
    termios::tcsetattr(&slave, OptionalActions::Now, &cooked).expect("set cooked output");
    termios::tcsetwinsize(
        &slave,
        Winsize {
            ws_row: 40,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        },
    )
    .expect("set pty size");

    let (input_read, mut input_write) = std::io::pipe().expect("input pipe");
    input_write
        .write_all(b"hello\nworld\n")
        .expect("write input");
    drop(input_write);

    let slave = std::fs::File::from(slave);
    // SAFETY: the child only calls setsid and TIOCSCTTY before exec.
    let mut child = unsafe {
        Command::new(env!("CARGO_BIN_EXE_scrl"))
            .arg("--paging=auto")
            .stdin(Stdio::from(input_read))
            .stdout(Stdio::from(slave))
            .stderr(Stdio::piped())
            .pre_exec(|| {
                rustix::process::setsid()?;
                // SAFETY: fd 1 is the slave dup'd onto stdout, open in this child.
                let stdout = BorrowedFd::borrow_raw(1);
                rustix::process::ioctl_tiocsctty(stdout)?;
                Ok(())
            })
            .spawn()
    }
    .expect("spawn scrl");

    let reader = thread::spawn(move || {
        let mut master = std::fs::File::from(master);
        let mut output = Vec::new();
        let mut buffer = [0; 256];
        loop {
            match master.read(&mut buffer) {
                Ok(0) => break output,
                Ok(count) => output.extend_from_slice(&buffer[..count]),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => panic!("read pty: {error}"),
            }
        }
    });

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait scrl") {
            break status;
        }
        if started.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            break child.wait().expect("reap scrl");
        }
        thread::sleep(Duration::from_millis(20));
    };
    let output = reader.join().expect("pty reader");
    let mut stderr = Vec::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_end(&mut stderr);
    }

    assert_eq!(
        (status.success(), output.as_slice()),
        (true, &b"hello\r\nworld\r\n"[..]),
        "stderr: {}",
        String::from_utf8_lossy(&stderr)
    );
}
