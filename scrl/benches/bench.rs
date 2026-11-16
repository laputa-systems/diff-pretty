//! Focused scrl pager benchmarks.
//!
//! These measure complete viewport redraws, horizontal clipping, cached search
//! highlighting, and a live document receiving a new chunk while search is
//! active. Setup happens outside the steady-state redraw benchmarks.
//! rustybench's allocator profiler reports time, allocation count, and
//! allocated bytes for each operation.

use rustybench::{AllocProfiler, Bencher, black_box};
use scrl::{Document, DocumentBuilder, Event, Session, SessionOptions, Size};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn corpus(lines: usize) -> String {
    let mut input = String::with_capacity(lines * 96);
    for line in 0..lines {
        input.push_str("commit ");
        input.push_str(&format!("{line:08x}"));
        input.push_str(" function render_document changed_token_0001 changed_token_0002\n");
    }
    input
}

fn styled_corpus(lines: usize) -> String {
    let mut input = String::with_capacity(lines * 120);
    for line in 0..lines {
        input.push_str("\x1b[31mremoved ");
        input.push_str(&format!("{line:08x}"));
        input.push_str("\x1b[0m context ");
        input.push_str("\x1b[32madded changed_token_0001\x1b[0m\n");
    }
    input
}

fn document(input: &str) -> Document {
    let mut builder = DocumentBuilder::new();
    builder.push_str(input);
    builder.finish()
}

fn session(document_text: &str) -> Session {
    let mut pager = Session::new(
        Size {
            rows: 24,
            columns: 100,
        },
        SessionOptions {
            title: "bench".into(),
            search_history: Vec::new(),
            wrap: false,
            follow: false,
            filter: None,
        },
    )
    .unwrap_or_else(|error| panic!("valid benchmark options: {error}"));
    pager.push_chunk(document_text);
    pager.finish();
    pager
}

#[rustybench::bench]
fn viewport_redraw_no_search(bencher: Bencher) {
    let document = document(&corpus(20_000));
    let mut frame = Vec::with_capacity(16 * 1024);
    bencher.bench_local(|| {
        frame.clear();
        document.write_viewport(&mut frame, 2_000, 24).unwrap();
        black_box(frame.len())
    });
}

#[rustybench::bench]
fn viewport_redraw_styled(bencher: Bencher) {
    let document = document(&styled_corpus(20_000));
    let mut frame = Vec::with_capacity(16 * 1024);
    bencher.bench_local(|| {
        frame.clear();
        document.write_viewport(&mut frame, 2_000, 24).unwrap();
        black_box(frame.len())
    });
}

#[rustybench::bench]
fn viewport_redraw_horizontal_clip(bencher: Bencher) {
    let mut pager = session(&corpus(20_000));
    pager.handle(Event::Right);
    let mut frame = Vec::with_capacity(16 * 1024);
    bencher.bench_local(|| {
        frame.clear();
        pager.draw(&mut frame).unwrap();
        black_box(frame.len())
    });
}

#[rustybench::bench]
fn session_redraw_without_movement(bencher: Bencher) {
    let text = corpus(20_000);
    let mut pager = session(&text);
    let mut frame = Vec::with_capacity(16 * 1024);
    bencher.bench_local(|| {
        frame.clear();
        pager.draw(&mut frame).unwrap();
        black_box(frame.len())
    });
}

#[rustybench::bench]
fn initial_search_late_match(bencher: Bencher) {
    let text = corpus(20_000);
    bencher.bench_local(|| {
        let mut pager = session(&text);
        pager.handle(Event::Text('/'));
        for character in "changed_token_0002".chars() {
            pager.handle(Event::Text(character));
        }
        pager.handle(Event::Enter);
        let mut frame = Vec::with_capacity(16 * 1024);
        pager.draw(&mut frame).unwrap();
        black_box(frame.len())
    });
}

#[rustybench::bench]
fn initial_regex_search_late_match(bencher: Bencher) {
    let text = corpus(20_000);
    bencher.bench_local(|| {
        let mut pager = session(&text);
        pager.handle(Event::Text('/'));
        for character in "changed_token_000[2]".chars() {
            pager.handle(Event::Text(character));
        }
        pager.handle(Event::Enter);
        let mut frame = Vec::with_capacity(16 * 1024);
        pager.draw(&mut frame).unwrap();
        black_box(frame.len())
    });
}

#[rustybench::bench]
fn cached_search_redraw(bencher: Bencher) {
    let text = corpus(20_000);
    let mut pager = session(&text);
    pager.handle(Event::Text('/'));
    for character in "changed_token_0001".chars() {
        pager.handle(Event::Text(character));
    }
    pager.handle(Event::Enter);
    let mut frame = Vec::with_capacity(16 * 1024);
    pager.draw(&mut frame).unwrap();
    bencher.bench_local(|| {
        frame.clear();
        pager.draw(&mut frame).unwrap();
        black_box(frame.len())
    });
}

#[rustybench::bench]
fn live_search_chunk_and_redraw(bencher: Bencher) {
    let first = corpus(10_000);
    let second = "late changed_token_0002\n".repeat(100);
    bencher.bench_local(|| {
        let mut pager = Session::new(
            Size {
                rows: 24,
                columns: 100,
            },
            SessionOptions {
                title: "bench".into(),
                search_history: Vec::new(),
                wrap: false,
                follow: false,
                filter: None,
            },
        )
        .unwrap_or_else(|error| panic!("valid benchmark options: {error}"));
        pager.push_chunk(&first);
        pager.handle(Event::Text('/'));
        for character in "changed_token_0002".chars() {
            pager.handle(Event::Text(character));
        }
        pager.handle(Event::Enter);
        pager.push_chunk(&second);
        pager.advance();
        let mut frame = Vec::with_capacity(16 * 1024);
        pager.draw(&mut frame).unwrap();
        black_box(frame.len())
    });
}

fn filtered_stream_and_redraw(bencher: Bencher, minimum_bytes: usize) {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures");
    let mut paths: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "patch"))
        .collect();
    paths.sort();
    let mut input = String::new();
    for path in paths {
        input.push_str(&std::fs::read_to_string(path).unwrap());
        input.push('\n');
        if input.len() >= minimum_bytes { break; }
    }
    assert!(!input.is_empty());
    let input = input.repeat(minimum_bytes.div_ceil(input.len()));
    let unfiltered = document(&input);
    let expected_lines = (0..unfiltered.line_count())
        .filter_map(|line| unfiltered.line_text(line))
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .count();
    assert!(expected_lines > 0);
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < input.len() {
        let mut end = (start + 4096).min(input.len());
        while !input.is_char_boundary(end) { end -= 1; }
        chunks.push(&input[start..end]);
        start = end;
    }
    bencher.bench_local(|| {
        let mut pager = Session::new(
            Size { rows: 24, columns: 100 },
            SessionOptions {
                filter: Some("^\\+($|[^+])".into()),
                ..SessionOptions::default()
            },
        ).unwrap();
        for chunk in &chunks { pager.push_chunk(chunk); }
        pager.finish();
        assert_eq!(pager.document().line_count(), expected_lines + 1);
        assert_eq!(pager.document().line_text(expected_lines), Some(""));
        pager.handle(Event::End);
        let mut frame = Vec::new();
        pager.draw(&mut frame).unwrap();
        assert!(!frame.is_empty());
        black_box((pager.document().len(), frame.len()))
    });
}

#[rustybench::bench]
fn filtered_stream_and_redraw_50k(bencher: Bencher) {
    filtered_stream_and_redraw(bencher, 50 * 1024);
}

#[rustybench::bench]
fn filtered_stream_and_redraw_500k(bencher: Bencher) {
    filtered_stream_and_redraw(bencher, 500 * 1024);
}

#[rustybench::bench]
fn filtered_stream_and_redraw_5m(bencher: Bencher) {
    filtered_stream_and_redraw(bencher, 5 * 1024 * 1024);
}

fn main() {
    rustybench::main();
}
