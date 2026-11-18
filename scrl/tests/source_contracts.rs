use scrl::{ChunkSource, Event, FileSource, FilesSource, ReaderSource, Session, SessionOptions, Size};
use std::fs;
use std::io::{self, BufRead, BufReader, Cursor, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

#[path = "../../tests/support/mod.rs"]
mod support;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

struct FailingReader { bytes: Vec<u8>, pos: usize, reads: Arc<AtomicUsize> }
impl Read for FailingReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let data = self.fill_buf()?;
        let n = out.len().min(data.len());
        out[..n].copy_from_slice(&data[..n]);
        self.consume(n);
        Ok(n)
    }
}
impl BufRead for FailingReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.pos == self.bytes.len() { Err(io::Error::other("read sentinel")) }
        else { Ok(&self.bytes[self.pos..]) }
    }
    fn consume(&mut self, n: usize) { self.pos += n; }
}

#[test]
fn source_contracts_bounded() {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.stdout(Stdio::piped()).args(["--exact", "source_contracts_child", "--nocapture"])
        .env("SCRL_SOURCE_CONTRACT_CHILD", "1");
    let output = support::run(command, None);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("source bootstrap"), "child did not start: {stdout}");
    assert!(output.status.success(), "child failed: {stdout} {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("source complete"), "child did not complete: {stdout}");
}

#[test]
fn source_contracts_child() {
    if std::env::var_os("SCRL_SOURCE_CONTRACT_CHILD").is_none() { return; }
    println!("source bootstrap");
    let input = "aé𐍈z\n終";
    for capacity in [1, 2, 3, 4, 7, 16384] {
        let reader = BufReader::with_capacity(capacity, Cursor::new(input.as_bytes()));
        let mut text = String::new();
        ReaderSource::new(reader).produce(&mut |s| {
            assert!(!s.is_empty(), "callback must carry data");
            text.push_str(s);
            Ok(())
        }).unwrap();
        assert_eq!(text, input, "capacity {capacity}");
    }
    for (bytes, prefix) in [(&b"ok\xe2\x82"[..], "ok"), (&b"yes\xe2\x82\xff"[..], "yes"), (&b"a\xc3\xa9!\xff"[..], "aé!")] {
        for capacity in [1, 2, 8] {
            let mut text = String::new();
            let err = ReaderSource::new(BufReader::with_capacity(capacity, Cursor::new(bytes)))
                .produce(&mut |s| { text.push_str(s); Ok(()) }).unwrap_err();
            assert_eq!(text, prefix);
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }
    }
    let reads = Arc::new(AtomicUsize::new(0));
    let reader = FailingReader { bytes: b"prefix".to_vec(), pos: 0, reads: reads.clone() };
    let mut text = String::new();
    let err = ReaderSource::new(reader).produce(&mut |s| { text.push_str(s); Ok(()) }).unwrap_err();
    assert_eq!(text, "prefix");
    assert_eq!(err.to_string(), "read sentinel");
    let reader = FailingReader { bytes: b"prefix".to_vec(), pos: 0, reads: reads.clone() };
    reads.store(0, Ordering::SeqCst);
    let mut calls = 0;
    let err = ReaderSource::new(reader).produce(&mut |_| { calls += 1; Err(io::Error::other("callback sentinel")) }).unwrap_err();
    assert_eq!(err.to_string(), "callback sentinel");
    assert_eq!(calls, 1);
    assert_eq!(reads.load(Ordering::SeqCst), 1);
    let mut text = String::new();
    let err = ReaderSource::new(Cursor::new(&b"ok\xff"[..])).produce(&mut |s| { text.push_str(s); Err(io::Error::other("callback first")) }).unwrap_err();
    assert_eq!(text, "ok");
    assert_eq!(err.to_string(), "callback first");

    let fixture = Fixture(std::env::temp_dir().join(format!("scrl-source-contract-{}", std::process::id())));
    fs::create_dir(&fixture.0).unwrap();
    let first = fixture.0.join("first");
    let second = fixture.0.join("second");
    let missing = fixture.0.join("missing");
    let owned = first.to_string_lossy().into_owned();
    let source = FileSource::new(&owned);
    drop(owned);
    assert_eq!(source.path(), first);
    fs::write(&first, "oneé").unwrap();
    let mut text = String::new();
    source.produce(&mut |s| { text.push_str(s); Ok(()) }).unwrap();
    assert_eq!(text, "oneé");
    let paths = vec![first.clone(), second.clone()];
    let source = FilesSource::new(&paths);
    drop(paths);
    fs::write(&second, "二end").unwrap();
    text.clear();
    source.produce(&mut |s| { text.push_str(s); Ok(()) }).unwrap();
    assert_eq!(text, "oneé二end");
    text.clear();
    let err = FilesSource::new([&first, &missing]).produce(&mut |s| { text.push_str(s); Ok(()) }).unwrap_err();
    assert_eq!(text, "oneé");
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert_eq!(FileSource::new(&missing).produce(&mut |_| Ok(())).unwrap_err().kind(), io::ErrorKind::NotFound);
    text.clear();
    FilesSource::new(Vec::<PathBuf>::new()).produce(&mut |s| { text.push_str(s); Ok(()) }).unwrap();
    assert_eq!(text, "");
    println!("source complete");
}

#[test]
fn finished_session_preserves_the_last_chunk_and_ignores_late_input() {
    let mut session = Session::new(
        Size { rows: 4, columns: 40 },
        SessionOptions::default(),
    ).unwrap();
    let input = "one\ntwo without newline";
    session.push_chunk(input);
    session.finish();
    session.push_chunk("\nlate input");

    let mut output = Vec::new();
    session.document().write_to(&mut output).unwrap();
    assert_eq!(output, input.as_bytes());
}

#[test]
fn upward_navigation_starts_at_the_clamped_view_after_a_filtered_tail_changes() {
    let mut session = Session::new(
        Size { rows: 2, columns: 40 },
        SessionOptions { filter: Some("match$".into()), ..SessionOptions::default() },
    ).unwrap();
    session.push_chunk("first match\nsecond match\nthird match");
    session.handle(Event::End);
    let mut frame = Vec::new();
    session.draw(&mut frame).unwrap();
    assert!(frame.starts_with(b"\x1b[H\x1b[2K\rthird match\x1b[0m"));

    session.push_chunk(" extended");
    session.handle(Event::Up);
    frame.clear();
    session.draw(&mut frame).unwrap();
    assert!(frame.starts_with(b"\x1b[H\x1b[2K\rfirst match\x1b[0m"));
}
