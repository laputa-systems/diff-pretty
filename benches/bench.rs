//! Curated rustybench benchmarks for diff-pretty's hot paths.
//!
//! What is curated here:
//!
//! - **End-to-end rendering** of representative Git-show, metadata-heavy,
//!   multi-commit, colorized, and plain unified-diff fixtures.
//! - **Real streaming and retained-document paths** over those same fixtures.
//! - **Real pager paths** using a checked-in document for viewport rendering and
//!   search setup/redraw.
//!
//! The real corpus is split into typical, metadata-heavy, and large bundles so
//! each practical shape is visible instead of being hidden by one aggregate.
//! Setup (fixture loads) happens once outside the measured closure, so the
//! medians and alloc counts are the steady-state cost
//! of one operation.

use std::path::PathBuf;
use std::sync::OnceLock;

use rustybench::{AllocProfiler, Bencher, black_box};

use diff_pretty::{render, render_document, render_reader_document, render_reader_to};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

/// Every `fixtures/*.patch`, sorted by name. Used directly by the corpus bench
/// and as the source for individual named fixtures.
fn corpus() -> &'static [(String, String)] {
    static CORPUS: OnceLock<Vec<(String, String)>> = OnceLock::new();
    CORPUS.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        let mut patches: Vec<(String, String)> = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "patch"))
            .map(|e| {
                let stem = e.path().file_stem().unwrap().to_str().unwrap().to_string();
                let input = std::fs::read_to_string(e.path()).unwrap();
                (stem, input)
            })
            .collect();
        patches.sort_by(|a, b| a.0.cmp(&b.0));
        patches
    })
}

fn fixture(name: &str) -> &'static str {
    corpus()
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, s)| s.as_str())
        .unwrap_or_else(|| panic!("fixture {name} not in corpus"))
}

const SHOW_METADATA_FIXTURES: &[&str] = &[
    "show_001", "show_003", "show_006", "show_032", "show_038", "show_039", "show_040", "show_063",
    "show_078", "show_082", "show_095", "show_098",
];

const SHOW_LARGE_FIXTURES: &[&str] = &[
    "show_015", "show_041", "show_048", "show_059", "show_079", "show_084", "show_088", "show_096",
    "show_099",
];

fn render_named_fixtures(names: &[&str]) -> usize {
    names.iter().map(|name| render(fixture(name)).len()).sum()
}

fn render_typical_show_fixtures() -> usize {
    corpus()
        .iter()
        .filter(|(name, _)| {
            name.starts_with("show_")
                && !SHOW_METADATA_FIXTURES.contains(&name.as_str())
                && !SHOW_LARGE_FIXTURES.contains(&name.as_str())
        })
        .map(|(_, input)| render(input).len())
        .sum()
}

#[rustybench::bench]
fn render_show_typical(b: Bencher) {
    b.bench_local(|| black_box(render_typical_show_fixtures()));
}

#[rustybench::bench]
fn render_show_metadata(b: Bencher) {
    b.bench_local(|| black_box(render_named_fixtures(SHOW_METADATA_FIXTURES)));
}

#[rustybench::bench]
fn render_show_large(b: Bencher) {
    b.bench_local(|| black_box(render_named_fixtures(SHOW_LARGE_FIXTURES)));
}

#[rustybench::bench]
fn render_show_corpus(b: Bencher) {
    b.bench_local(|| {
        black_box(
            render_typical_show_fixtures()
                .wrapping_add(render_named_fixtures(SHOW_METADATA_FIXTURES))
                .wrapping_add(render_named_fixtures(SHOW_LARGE_FIXTURES)),
        )
    });
}

#[rustybench::bench]
fn render_document_show_010(b: Bencher) {
    let input = fixture("show_010");
    b.bench_local(|| black_box(render_document(input)));
}

#[rustybench::bench]
fn render_reader_to_log_000(b: Bencher) {
    let input = fixture("log_000");
    b.bench_local(|| {
        let mut output = std::io::sink();
        render_reader_to(input.as_bytes(), &mut output).expect("sink writes cannot fail");
        black_box(output)
    });
}

#[rustybench::bench]
fn render_log_000(b: Bencher) {
    let input = fixture("log_000");
    b.bench_local(|| black_box(render(input).len()));
}

#[rustybench::bench]
fn render_log_000_color(b: Bencher) {
    let input = fixture("log_000_color");
    b.bench_local(|| black_box(render(input).len()));
}

#[rustybench::bench]
fn render_plain_unified(b: Bencher) {
    let input = fixture("plain_unified");
    b.bench_local(|| black_box(render(input).len()));
}

fn synthetic_git_log(commits: usize) -> String {
    let mut input = String::with_capacity(commits * 160);
    for commit in 0..commits {
        input.push_str(&format!(
            "commit {commit:07x}\nAuthor: Synthetic <synthetic@example.test>\nDate:   Thu Jan 1 00:00:00 1970 +0000\n\n    synthetic commit {commit}\n\n"
        ));
    }
    input
}

#[rustybench::bench]
fn render_reader_document_synthetic_git_log_10000(b: Bencher) {
    let input = synthetic_git_log(10_000);
    b.bench_local(|| black_box(render_reader_document(input.as_bytes()).unwrap()));
}

#[rustybench::bench]
fn pager_viewport_show_010(b: Bencher) {
    let document = render_document(fixture("show_010"));
    let mut terminal = Vec::with_capacity(16 * 1024);
    b.bench_local(|| {
        terminal.clear();
        document
            .write_viewport(&mut terminal, 0, 24)
            .expect("writing to a Vec cannot fail");
        black_box(terminal.len())
    });
}

fn main() {
    rustybench::main();
}

fn cli_binary() -> PathBuf {
    std::env::var_os("DIFF_PRETTY_BENCH_BINARY").map(PathBuf::from).unwrap_or_else(|| {
        std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"))
            .join("release").join("diff-pretty")
    })
}

#[rustybench::bench]
fn cli_render_show_010(b: Bencher) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let input = fixture("show_010");
    let expected = render(input);
    b.bench_local(|| {
        let mut child = Command::new(cli_binary()).arg("--paging=never")
            .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, expected.as_bytes(), "CLI output differs from render oracle");
        black_box(output.stdout)
    });
}

#[cfg(unix)]
#[rustybench::bench]
fn cli_pager_show_010(b: Bencher) {
    use std::io::{Read, Write};
    use std::os::fd::BorrowedFd;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    use rustix::fs::{Mode, OFlags};
    use rustix::pty::OpenptFlags;
    use rustix::termios::{self, OptionalActions, Winsize};

    struct OwnedChild(std::process::Child);
    impl Drop for OwnedChild {
        fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); }
    }
    let input = fixture("show_010");
    let rendered = render(input);
    assert!(rendered.contains("Author:"));
    assert!(rendered.contains("extern crate"));
    assert!(rendered.contains("println!"));
    b.bench_local(|| {
        let master = rustix::pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).unwrap();
        rustix::fs::fcntl_setfl(&master, OFlags::NONBLOCK).unwrap();
        rustix::pty::grantpt(&master).unwrap();
        rustix::pty::unlockpt(&master).unwrap();
        let name = rustix::pty::ptsname(&master, Vec::new()).unwrap();
        let slave = rustix::fs::open(name.as_c_str(), OFlags::RDWR | OFlags::NOCTTY, Mode::empty()).unwrap();
        let original = termios::tcgetattr(&slave).unwrap();
        termios::tcsetattr(&slave, OptionalActions::Now, &original).unwrap();
        termios::tcsetwinsize(&slave, Winsize { ws_row: 10, ws_col: 100, ws_xpixel: 0, ws_ypixel: 0 }).unwrap();
        let slave = std::fs::File::from(slave);
        let (reader, mut writer) = std::io::pipe().unwrap();
        writer.write_all(input.as_bytes()).unwrap();
        drop(writer);
        // SAFETY: the child only calls async-signal-safe operations before exec; stdout is the PTY slave.
        let child = unsafe { Command::new(cli_binary()).arg("--paging=always")
            .stdin(Stdio::from(reader)).stdout(Stdio::from(slave)).stderr(Stdio::piped())
            .pre_exec(|| {
                rustix::process::setsid()?;
                rustix::process::ioctl_tiocsctty(BorrowedFd::borrow_raw(1))?;
                Ok(())
            }).spawn().unwrap() };
        let mut child = OwnedChild(child);
        let mut master = std::fs::File::from(master);
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut output = Vec::new();
        for (needle, action) in [(b"Author:".as_slice(), b" ".as_slice()), (b"extern crate", b"/println!\n"), (b"\x1b[48;5;226;30mprintln!", b"q")] {
            let mut frame = Vec::new();
            while !frame.windows(needle.len()).any(|w| w == needle) {
                let mut buf = [0; 8192];
                match master.read(&mut buf) {
                    Ok(n) => frame.extend_from_slice(&buf[..n]),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::Interrupted => {},
                    Err(e) => panic!("pty read: {e}"),
                }
                assert!(child.0.try_wait().unwrap().is_none(), "pager exited before expected frame");
                assert!(Instant::now() < deadline, "pager deadline waiting for {:?}: {:?}", needle, String::from_utf8_lossy(&frame));
                std::thread::sleep(Duration::from_millis(1));
            }
            output.extend_from_slice(&frame);
            master.write_all(action).unwrap();
        }
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() { break status; }
            let mut buf = [0; 8192];
            match master.read(&mut buf) {
                Ok(n) => output.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::Interrupted => {},
                Err(e) => panic!("pty read: {e}"),
            }
            assert!(Instant::now() < deadline, "pager did not exit");
            std::thread::sleep(Duration::from_millis(1));
        };
        assert!(status.success());
        let restored = termios::tcgetattr(&master).unwrap();
        assert_eq!(restored.local_modes, original.local_modes);
        assert_eq!(restored.output_modes, original.output_modes);
        black_box(output)
    });
}
