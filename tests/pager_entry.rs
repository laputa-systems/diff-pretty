#![cfg(unix)]

use std::io::{Read, Write};
use std::os::fd::BorrowedFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use rustix::fs::{Mode, OFlags};
use rustix::pty::OpenptFlags;
use rustix::termios::{self, LocalModes, OptionalActions, OutputModes, Winsize};

const PATCH: &[u8] = b"diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n";
const OUTPUT: &str = concat!(
    "\r\n\x1b[34mΔ a\x1b[0m\r\n",
    "\x1b[34m────────────────────────────────────────────────────────────────────────────────\x1b[0m\r\n\r\n",
    "\x1b[34m\x1b[38;5;88m  1 \x1b[34m\x1b[38;5;28m    \x1b[34m\x1b[31mold\x1b[0m\r\n",
    "\x1b[34m\x1b[38;5;88m    \x1b[34m\x1b[38;5;28m  1 \x1b[34m\x1b[32mnew\x1b[0m\r\n",
);

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); }
}

fn exercise(mode: &str, interactive: bool, read_error: bool) {
    let master = rustix::pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).unwrap();
    rustix::fs::fcntl_setfl(&master, OFlags::NONBLOCK).unwrap();
    rustix::pty::grantpt(&master).unwrap();
    rustix::pty::unlockpt(&master).unwrap();
    let name = rustix::pty::ptsname(&master, Vec::new()).unwrap();
    let slave = rustix::fs::open(name.as_c_str(), OFlags::RDWR | OFlags::NOCTTY, Mode::empty()).unwrap();
    let mut original = termios::tcgetattr(&slave).unwrap();
    original.output_modes = OutputModes::OPOST | OutputModes::ONLCR;
    original.local_modes.insert(LocalModes::ISIG | LocalModes::ICANON | LocalModes::ECHO);
    termios::tcsetattr(&slave, OptionalActions::Now, &original).unwrap();
    termios::tcsetwinsize(&slave, Winsize { ws_row: 40, ws_col: 100, ws_xpixel: 0, ws_ypixel: 0 }).unwrap();
    let slave_file = std::fs::File::from(slave);
    let (input, mut writer) = std::io::pipe().unwrap();
    writer.write_all(PATCH).unwrap();
    drop(writer);
    // SAFETY: pre_exec performs only async-signal-safe setsid and ioctl on the child's stdout, which is the open PTY slave.
    let stdin = if read_error { Stdio::from(std::fs::File::open(env!("CARGO_MANIFEST_DIR")).unwrap()) } else { Stdio::from(input) };
    let child = unsafe { Command::new(env!("CARGO_BIN_EXE_diff-pretty"))
        .arg(format!("--paging={mode}"))
        .env("PAGER", "/nonexistent/pager")
        .stdin(stdin).stdout(Stdio::from(slave_file.try_clone().unwrap())).stderr(Stdio::piped())
        .pre_exec(|| {
            rustix::process::setsid()?;
            // SAFETY: fd 1 is the PTY slave installed as stdout for this child before exec.
            rustix::process::ioctl_tiocsctty(BorrowedFd::borrow_raw(1))?;
            Ok(())
        }).spawn().unwrap() };
    let mut child = OwnedChild(child);
    let mut master = std::fs::File::from(master);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut output = Vec::new();
    let mut sent = false;
    let status: ExitStatus = loop {
        let mut buf = [0; 4096];
        match master.read(&mut buf) {
            Ok(n) if n > 0 => output.extend_from_slice(&buf[..n]),
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::Interrupted => {},
            Err(e) => panic!("pty read: {e}"),
        }
        if interactive && !sent && output.windows(3).any(|w| w == b"new") && output.windows(8).any(|w| w == b"\x1b[?1049h") {
            master.write_all(b"q").unwrap();
            sent = true;
        }
        if let Some(status) = child.0.try_wait().unwrap() { break status; }
        assert!(Instant::now() < deadline, "child deadline; output: {:?}", String::from_utf8_lossy(&output));
        std::thread::sleep(Duration::from_millis(5));
    };
    loop {
        let mut buf = [0; 4096];
        match master.read(&mut buf) {
            Ok(n) if n > 0 => output.extend_from_slice(&buf[..n]),
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => panic!("pty read: {e}"),
        }
    }
    let mut stderr = Vec::new();
    child.0.stderr.take().unwrap().read_to_end(&mut stderr).unwrap();
    if read_error {
        assert_eq!(status.code(), Some(1));
        assert!(stderr.starts_with(b"failed to write output: "));
        assert!(output.is_empty());
    } else {
        assert_eq!(status.code(), Some(0), "stderr: {:?}", String::from_utf8_lossy(&stderr));
        assert!(stderr.is_empty());
    }
    let restored = termios::tcgetattr(&master).unwrap();
    assert_eq!(restored.local_modes, original.local_modes);
    assert_eq!(restored.output_modes, original.output_modes);
    if read_error {
        return;
    }
    if interactive {
        assert!(sent, "payload not observed: {:?}", String::from_utf8_lossy(&output));
        assert!(output.windows(8).any(|w| w == b"\x1b[?1049l"));
        assert!(output.windows(6).any(|w| w == b"\x1b[?25h"));
        assert!(output.windows(3).any(|w| w == b"old"));
    } else {
        assert_eq!(output, OUTPUT.as_bytes());
    }
}

#[test]
fn auto_short_diff_is_cooked_direct_output() { exercise("auto", false, false); }
#[test]
fn never_is_cooked_direct_output() { exercise("never", false, false); }
#[test]
fn always_enters_and_restores_terminal() { exercise("always", true, false); }

#[test]
fn auto_reports_source_read_error() { exercise("auto", false, true); }
