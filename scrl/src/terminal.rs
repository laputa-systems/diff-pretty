use std::io::{self, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::Event;

#[cfg(target_os = "linux")]
const SIGTSTP: std::ffi::c_int = 20;
#[cfg(target_os = "macos")]
const SIGTSTP: std::ffi::c_int = 18;
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
compile_error!("scrl terminal signal handling supports Linux and macOS");

#[cfg(unix)]
static TERMINATED: AtomicBool = AtomicBool::new(false);
#[cfg(unix)]
static SUSPEND_REQUESTED: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
// SAFETY: `signal` and `raise` are the POSIX functions with these signatures.
unsafe extern "C" {
    fn signal(signal: std::ffi::c_int, handler: usize) -> usize;
    fn raise(signal: std::ffi::c_int) -> std::ffi::c_int;
}

#[cfg(unix)]
// SAFETY: the handler only stores an atomic. That is async-signal-safe.
unsafe extern "C" fn handle_term(_signal: std::ffi::c_int) {
    TERMINATED.store(true, Ordering::Relaxed);
}

#[cfg(unix)]
// SAFETY: the handler only stores an atomic. That is async-signal-safe.
unsafe extern "C" fn handle_tstp(_signal: std::ffi::c_int) {
    SUSPEND_REQUESTED.store(true, Ordering::Relaxed);
}

#[cfg(unix)]
pub(crate) struct SignalGuard {
    previous_term: usize,
    previous_tstp: usize,
}

#[cfg(unix)]
impl SignalGuard {
    pub(crate) fn install() -> io::Result<Self> {
        TERMINATED.store(false, Ordering::Relaxed);
        SUSPEND_REQUESTED.store(false, Ordering::Relaxed);
        // SAFETY: both handlers only store atomics. Drop restores these dispositions.
        let previous_term = unsafe { signal(15, handle_term as *const () as usize) };
        // SAFETY: `usize::MAX` is `SIG_ERR`; no installed disposition compares equal.
        if previous_term == usize::MAX {
            return Err(io::Error::last_os_error());
        }
        let previous_tstp = unsafe { signal(SIGTSTP, handle_tstp as *const () as usize) };
        if previous_tstp == usize::MAX {
            let error = io::Error::last_os_error();
            // SAFETY: restores the disposition captured above.
            unsafe {
                signal(15, previous_term);
            }
            return Err(error);
        }
        Ok(Self {
            previous_term,
            previous_tstp,
        })
    }
}

/// Restore the default SIGTERM disposition and deliver SIGTERM to this
/// process. Call only after the terminal is restored: raw mode is off and
/// the alternate screen is gone, so dying here leaves nothing behind. When
/// the signal is blocked and `raise` returns, the caller still reports an
/// error instead of exiting zero.
#[cfg(unix)]
pub(crate) fn reraise_term() {
    // SAFETY: `0` is `SIG_DFL`. The terminal is already restored, so the
    // default disposition cannot strand raw mode or the alternate screen.
    unsafe {
        signal(15, 0);
        raise(15);
    }
}

#[cfg(unix)]
impl Drop for SignalGuard {
    fn drop(&mut self) {
        // SAFETY: these are the dispositions `install` captured from `signal`.
        unsafe {
            signal(15, self.previous_term);
            signal(SIGTSTP, self.previous_tstp);
        }
    }
}

#[cfg(unix)]
pub(crate) fn terminated() -> bool {
    TERMINATED.load(Ordering::Relaxed)
}

#[cfg(unix)]
pub(crate) fn suspend_requested() -> bool {
    SUSPEND_REQUESTED.swap(false, Ordering::Relaxed)
}

#[cfg(unix)]
pub(crate) fn suspend<W: io::Write + ?Sized>(output: &mut W, raw: &RawMode<'_>) -> io::Result<()> {
    output.write_all(b"\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l")?;
    output.flush()?;
    raw.restore();
    // SAFETY: `0` is `SIG_DFL`. `raise` delivers SIGTSTP after the terminal is restored.
    unsafe {
        signal(SIGTSTP, 0);
        raise(SIGTSTP);
    }
    raw.reenter()?;
    // SAFETY: reinstalls the atomic-only handler after the default disposition ran.
    unsafe {
        signal(SIGTSTP, handle_tstp as *const () as usize);
    }
    output.write_all(b"\x1b[?1049h\x1b[?7l\x1b[?25l\x1b[H")?;
    output.flush()
}

#[cfg(test)]
pub(crate) fn read_event<R: Read>(input: &mut R) -> io::Result<Option<Event>> {
    read_event_inner(input, false, None)
}

#[cfg(unix)]
pub(crate) fn read_event_tty_cancellable<R: Read>(
    input: &mut R,
    cancelled: &AtomicBool,
) -> io::Result<Option<Event>> {
    read_event_inner(input, true, Some(cancelled))
}

fn read_event_inner<R: Read>(
    input: &mut R,
    timeout_is_idle: bool,
    cancelled: Option<&AtomicBool>,
) -> io::Result<Option<Event>> {
    let mut decoder = KeyDecoder::new();
    let mut byte = [0; 1];
    loop {
        let count = input.read(&mut byte)?;
        if count == 0 {
            if !timeout_is_idle {
                return Ok(decoder.finish());
            }
            if cancelled.is_some_and(|cancelled| cancelled.load(Ordering::Relaxed)) {
                return Ok(None);
            }
            if let Some(event) = decoder.timed_out(Instant::now()) {
                return Ok(Some(event));
            }
            continue;
        }
        decoder.push(byte[0]);
        if let Some(event) = decoder.next() {
            return Ok(Some(event));
        }
    }
}

/// Decodes terminal bytes without assuming that an escape sequence or UTF-8
/// scalar arrives in one read. The fixed storage bounds malformed input and
/// lets the terminal adapter retain a decoder across reads.
pub(crate) struct KeyDecoder {
    bytes: [u8; 64],
    len: usize,
}

impl KeyDecoder {
    pub(crate) fn new() -> Self {
        Self {
            bytes: [0; 64],
            len: 0,
        }
    }

    pub(crate) fn push(&mut self, byte: u8) {
        if self.len == self.bytes.len() {
            self.consume(1);
        }
        self.bytes[self.len] = byte;
        self.len += 1;
    }

    pub(crate) fn next(&mut self) -> Option<Event> {
        if self.len == 0 {
            return None;
        }
        match self.bytes[0] {
            3 => {
                self.consume(1);
                Some(Event::Interrupt)
            }
            b'\n' | b'\r' => {
                self.consume(1);
                Some(Event::Enter)
            }
            8 | 127 => {
                self.consume(1);
                Some(Event::Backspace)
            }
            0x15 => {
                self.consume(1);
                Some(Event::CtrlU)
            }
            0x20..=0x7e => {
                let character = self.bytes[0] as char;
                self.consume(1);
                Some(Event::Text(character))
            }
            0x1b => self.escape_event(),
            _ => self.utf8_event(),
        }
    }

    pub(crate) fn finish(&mut self) -> Option<Event> {
        if self.len == 0 {
            return None;
        }
        self.consume(1);
        Some(Event::Escape)
    }

    pub(crate) fn timed_out(&mut self, _at: Instant) -> Option<Event> {
        // The raw tty uses VTIME below, so a zero-byte read is the deadline.
        // Keeping the clock at this seam makes the policy explicit and keeps
        // the decoder usable with adapters that implement a finer deadline.
        self.finish()
    }

    fn escape_event(&mut self) -> Option<Event> {
        if self.len == 1 {
            return None;
        }
        if self.bytes[1] != b'[' {
            self.consume(1);
            return Some(Event::Escape);
        }
        let final_index = (2..self.len).find(|&index| (0x40..=0x7e).contains(&self.bytes[index]));
        let final_index = final_index?;
        let sequence = &self.bytes[2..final_index];
        let event = match self.bytes[final_index] {
            b'A' if sequence.is_empty() => Event::Up,
            b'B' if sequence.is_empty() => Event::Down,
            b'C' if sequence.is_empty() => Event::Right,
            b'D' if sequence.is_empty() => Event::Left,
            b'H' if sequence.is_empty() => Event::Home,
            b'F' if sequence.is_empty() => Event::End,
            b'~' => match sequence {
                b"1" | b"7" => Event::Home,
                b"4" | b"8" => Event::End,
                b"3" => Event::Delete,
                b"5" => Event::PageUp,
                b"6" => Event::PageDown,
                _ => Event::Escape,
            },
            _ => Event::Escape,
        };
        self.consume(final_index + 1);
        Some(event)
    }

    fn utf8_event(&mut self) -> Option<Event> {
        match std::str::from_utf8(&self.bytes[..self.len]) {
            Ok(text) => {
                let character = text.chars().next()?;
                self.consume(character.len_utf8());
                Some(Event::Text(character))
            }
            Err(error) if error.error_len().is_none() => None,
            Err(_) => {
                self.consume(1);
                Some(Event::Escape)
            }
        }
    }

    fn consume(&mut self, count: usize) {
        self.bytes.copy_within(count..self.len, 0);
        self.len -= count;
    }
}

#[cfg(unix)]
pub(crate) struct RawMode<'a> {
    tty: &'a std::fs::File,
    original: rustix::termios::Termios,
}

#[cfg(unix)]
impl<'a> RawMode<'a> {
    pub(crate) fn enter(tty: &'a std::fs::File) -> io::Result<Self> {
        use rustix::termios::{self, OptionalActions};
        let original = termios::tcgetattr(tty)?;
        let mut raw = original.clone();
        raw.make_raw();
        raw.special_codes[termios::SpecialCodeIndex::VMIN] = 0;
        raw.special_codes[termios::SpecialCodeIndex::VTIME] = 1;
        termios::tcsetattr(tty, OptionalActions::Now, &raw)?;
        Ok(Self { tty, original })
    }

    pub(crate) fn restore(&self) {
        let _ = rustix::termios::tcsetattr(
            self.tty,
            rustix::termios::OptionalActions::Now,
            &self.original,
        );
    }

    pub(crate) fn reenter(&self) -> io::Result<()> {
        use rustix::termios::{self, OptionalActions};
        let mut raw = self.original.clone();
        raw.make_raw();
        raw.special_codes[termios::SpecialCodeIndex::VMIN] = 0;
        raw.special_codes[termios::SpecialCodeIndex::VTIME] = 1;
        termios::tcsetattr(self.tty, OptionalActions::Now, &raw)?;
        Ok(())
    }
}

#[cfg(unix)]
impl Drop for RawMode<'_> {
    fn drop(&mut self) {
        let _ = rustix::termios::tcsetattr(
            self.tty,
            rustix::termios::OptionalActions::Now,
            &self.original,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_terminal_keys() {
        let cases: &[(&[u8], Event)] = &[
            (b"\x03", Event::Interrupt),
            (b"\n", Event::Enter),
            (b"\r", Event::Enter),
            (b"\x08", Event::Backspace),
            (b"\x7f", Event::Backspace),
            (b"\x15", Event::CtrlU),
            (b"q", Event::Text('q')),
            (b"\x1b[A", Event::Up),
            (b"\x1b[B", Event::Down),
            (b"\x1b[C", Event::Right),
            (b"\x1b[D", Event::Left),
            (b"\x1b[H", Event::Home),
            (b"\x1b[F", Event::End),
            (b"\x1b[1~", Event::Home),
            (b"\x1b[7~", Event::Home),
            (b"\x1b[4~", Event::End),
            (b"\x1b[8~", Event::End),
            (b"\x1b[3~", Event::Delete),
            (b"\x1b[5~", Event::PageUp),
            (b"\x1b[6~", Event::PageDown),
        ];
        for &(bytes, event) in cases {
            let mut input = bytes;
            assert_eq!(read_event(&mut input).unwrap(), Some(event), "{bytes:?}");
        }
    }

    #[test]
    fn unsupported_terminal_sequences_end_as_escape() {
        let cases: &[&[u8]] = &[
            b"\x1b[1;5A", b"\x1b[1;5B", b"\x1b[1;5C", b"\x1b[1;5D", b"\x1b[1;5H",
            b"\x1b[1;5F", b"\x1b[9~",
        ];
        for &bytes in cases {
            let mut input = bytes;
            assert_eq!(read_event(&mut input).unwrap(), Some(Event::Escape), "{bytes:?}");
        }
    }

    #[test]
    fn decoder_consumes_invalid_utf8_and_preserves_following_text() {
        let mut decoder = KeyDecoder::new();
        decoder.push(0xff);
        decoder.push(b'q');
        assert_eq!(decoder.next(), Some(Event::Escape));
        assert_eq!(decoder.next(), Some(Event::Text('q')));
        assert_eq!(decoder.next(), None);
    }

    #[test]
    fn reader_stops_at_eof() {
        struct Eof { reads: usize }

        impl Read for Eof {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                self.reads += 1;
                assert_eq!(self.reads, 1, "read after EOF");
                Ok(0)
            }
        }

        assert_eq!(read_event(&mut Eof { reads: 0 }).unwrap(), None);
    }

    #[test]
    fn decoder_handles_split_sequences_and_navigation() {
        let mut decoder = KeyDecoder::new();
        decoder.push(0x1b);
        assert_eq!(decoder.next(), None);
        decoder.push(b'[');
        decoder.push(b'6');
        decoder.push(b'~');
        assert_eq!(decoder.next(), Some(Event::PageDown));

        for byte in "é".as_bytes().iter().copied().take(1) {
            decoder.push(byte);
        }
        assert_eq!(decoder.next(), None);
        decoder.push("é".as_bytes()[1]);
        assert_eq!(decoder.next(), Some(Event::Text('é')));
    }

    #[test]
    fn decoder_recognizes_home_end_delete_and_bare_escape() {
        let mut decoder = KeyDecoder::new();
        for byte in b"\x1b[H\x1b[F\x1b[3~" {
            decoder.push(*byte);
            if let Some(event) = decoder.next() {
                assert!(matches!(event, Event::Home | Event::End | Event::Delete));
            }
        }
        decoder.push(0x1b);
        assert_eq!(decoder.next(), None);
        assert_eq!(decoder.timed_out(Instant::now()), Some(Event::Escape));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn suspend_signal_matches_darwin() {
        assert_eq!(SIGTSTP, 18);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn suspend_signal_matches_linux() {
        assert_eq!(SIGTSTP, 20);
    }

    #[cfg(unix)]
    #[test]
    fn raw_mode_restores_and_reenters_on_the_same_terminal() {
        use rustix::fs::{Mode, OFlags};
        use rustix::pty::{self, OpenptFlags};
        use rustix::termios::{self, LocalModes, OptionalActions};

        let master = pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).unwrap();
        pty::grantpt(&master).unwrap();
        pty::unlockpt(&master).unwrap();
        let path = pty::ptsname(&master, Vec::new()).unwrap();
        let slave = rustix::fs::open(&path, OFlags::RDWR | OFlags::NOCTTY, Mode::empty()).unwrap();
        let slave = std::fs::File::from(slave);
        let cooked_flags = LocalModes::ISIG | LocalModes::ICANON | LocalModes::ECHO;
        let mut cooked = termios::tcgetattr(&slave).unwrap();
        cooked.local_modes.insert(cooked_flags);
        termios::tcsetattr(&slave, OptionalActions::Now, &cooked).unwrap();

        let modes = || termios::tcgetattr(&slave).unwrap().local_modes & cooked_flags;
        let raw = RawMode::enter(&slave).unwrap();
        assert_eq!(modes(), LocalModes::empty());
        let error = suspend(&mut &mut [][..], &raw).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WriteZero);
        assert_eq!(modes(), LocalModes::empty());
        raw.restore();
        assert_eq!(modes(), cooked_flags);
        raw.reenter().unwrap();
        assert_eq!(modes(), LocalModes::empty());
        drop(raw);
        assert_eq!(modes(), cooked_flags);
    }

    #[cfg(unix)]
    #[test]
    fn signal_guard_restores_previous_handlers() {
        const CHILD: &str = "SCRL_TEST_SIGNAL_GUARD_CHILD";
        const COMPLETED: i32 = 42;
        if std::env::var_os(CHILD).is_none() {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "terminal::tests::signal_guard_restores_previous_handlers"])
                .env(CHILD, "1")
                .spawn()
                .unwrap();
            let deadline = Instant::now() + std::time::Duration::from_secs(5);
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        assert_eq!(status.code(), Some(COMPLETED), "signal test child: {status}");
                        return;
                    }
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    result => {
                        let _ = child.kill();
                        let _ = child.wait();
                        panic!("signal test child did not complete: {result:?}");
                    }
                }
            }
        }

        unsafe extern "C" fn previous_handler(_number: std::ffi::c_int) {}

        // SAFETY: these handlers do nothing, and the dispositions are local to the child.
        unsafe {
            assert_ne!(signal(15, previous_handler as *const () as usize), usize::MAX);
            assert_ne!(signal(SIGTSTP, previous_handler as *const () as usize), usize::MAX);
        }
        let guard = SignalGuard::install().unwrap();
        // SAFETY: SIGTSTP is handled in this child and only sets an atomic.
        assert_eq!(unsafe { raise(SIGTSTP) }, 0);
        assert!(suspend_requested());
        assert!(!suspend_requested());

        drop(guard);
        // SAFETY: both signals were given handlers in this child before installing the guard.
        unsafe {
            assert_eq!(raise(15), 0);
            assert_eq!(raise(SIGTSTP), 0);
        }
        assert!(!terminated());
        assert!(!suspend_requested());
        std::process::exit(COMPLETED);
    }

    #[cfg(unix)]
    #[test]
    fn idle_tty_read_observes_cancellation_after_timeout() {
        struct CancellingIdleReader<'a> {
            cancelled: &'a AtomicBool,
            reads: usize,
        }

        impl Read for CancellingIdleReader<'_> {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                self.reads += 1;
                self.cancelled.store(true, Ordering::Relaxed);
                Ok(0)
            }
        }

        let cancelled = AtomicBool::new(false);
        let mut input = CancellingIdleReader {
            cancelled: &cancelled,
            reads: 0,
        };
        assert_eq!(read_event_tty_cancellable(&mut input, &cancelled).unwrap(), None);
        assert_eq!(input.reads, 1);
    }
}
