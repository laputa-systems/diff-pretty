//! Diff-specific source integration for the generic scrl runner.

use std::io::{self, BufRead, IsTerminal, Write};

pub use scrl::PagingMode;
use scrl::{ChunkSource, ExitReason, RunOptions, SessionOptions};

pub fn should_use_pager(mode: PagingMode) -> bool {
    matches!(mode, PagingMode::Always | PagingMode::Auto) && io::stdout().is_terminal()
}

pub fn emit_reader<R: BufRead + Send + 'static>(input: R, mode: PagingMode) -> io::Result<()> {
    if !should_use_pager(mode) {
        let stdout = io::stdout();
        let mut output = stdout.lock();
        crate::render::render_reader_to(input, &mut output)?;
        output.flush()?;
        return Ok(());
    }
    let options = RunOptions {
        paging: mode,
        session: SessionOptions {
            title: "diff-pretty".into(),
            search_history: Vec::new(),
            wrap: false,
            follow: false,
            filter: None,
        },
    };
    scrl::run_source(DiffSource { input }, options).map(|_: ExitReason| ())
}

struct DiffSource<R> {
    input: R,
}

impl<R: BufRead + Send + 'static> ChunkSource for DiffSource<R> {
    fn produce(self, emit: &mut dyn FnMut(&str) -> io::Result<()>) -> io::Result<()> {
        let mut input = self.input;
        crate::render::for_each_rendered_chunk(&mut input, emit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor, Read};

    #[test]
    fn pager_policy_matches_stdout_and_mode() {
        let terminal = io::stdout().is_terminal();
        assert!(!should_use_pager(PagingMode::Never));
        assert_eq!(should_use_pager(PagingMode::Auto), terminal);
        assert_eq!(should_use_pager(PagingMode::Always), terminal);
    }

    #[test]
    fn fragmented_input_emits_file_chunks_and_finishes() {
        let input = b"before\ndiff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n";
        let source = DiffSource { input: BufReader::with_capacity(2, Cursor::new(input)) };
        let mut chunks = Vec::new();
        source.produce(&mut |chunk| { chunks.push(chunk.to_owned()); Ok(()) }).unwrap();
        assert_eq!(chunks, ["before\n", concat!(
            "\n\x1b[34mΔ a\x1b[0m\n",
            "\x1b[34m────────────────────────────────────────────────────────────────────────────────\x1b[0m\n\n",
            "\x1b[34m\x1b[38;5;88m  1 \x1b[34m\x1b[38;5;28m    \x1b[34m\x1b[31mold\x1b[0m\n",
            "\x1b[34m\x1b[38;5;88m    \x1b[34m\x1b[38;5;28m  1 \x1b[34m\x1b[32mnew\x1b[0m\n",
        )]);
    }

    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("read sentinel"))
        }
    }

    #[test]
    fn read_and_callback_errors_propagate_and_stop() {
        let mut calls = 0;
        let error = DiffSource { input: BufReader::new(Broken) }.produce(&mut |_| {
            calls += 1;
            Ok(())
        }).unwrap_err();
        assert_eq!(error.to_string(), "read sentinel");
        assert_eq!(calls, 0);
        let input = Cursor::new(b"before\ndiff --git a/a b/a\nmore\ndiff --git a/b b/b\n".to_vec());
        let error = DiffSource { input }.produce(&mut |chunk| {
            calls += 1;
            assert_eq!(chunk, "before\n");
            Err(io::Error::other("callback sentinel"))
        }).unwrap_err();
        assert_eq!(error.to_string(), "callback sentinel");
        assert_eq!(calls, 1);
    }
}
