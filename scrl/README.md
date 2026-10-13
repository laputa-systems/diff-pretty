# scrl

`scrl` is an ANSI-aware terminal pager and Rust library for UTF-8 text. It
reads stdin or file operands, supports visible-text search, and displays a
terminal viewport without invoking an external pager.

## Install and run

Install the standalone command from this repository with Cargo (requires Rust):

```sh
cargo install --git https://github.com/laputa-systems/diff-pretty --package scrl
```

```sh
scrl < output.txt
scrl --paging=never file1.txt file2.txt
```

With no files, `scrl` reads stdin. Multiple files are streamed in order without
adding separators. The default paging mode is `auto`: use the pager when stdout
is a terminal and the content exceeds one screen. `--paging=always` requests
the pager when a usable terminal is available; `--paging=never` and
`--no-pager` write directly.

Options are `--wrap`, `--follow`, and `--filter=REGEX`. Wrapping uses Unicode
cell widths and eight-column tab stops. Without wrapping, long lines are
clipped at the terminal edge and Left/Right scroll horizontally. `--follow`
keeps the viewport at the newest input; `--filter` displays matching logical
lines.

Navigation includes arrows, `j`/`k`, Page Up/Down, Home, End, `g`/`G`, `b`,
Space, and Left/Right. Press `q` or Ctrl-C to quit; `h` opens help. Press `/` or
`?` to search forward or backward, Enter to submit, and `n`/`N` to move between
matches. Search uses visible line text, not ANSI styling.

Terminal dimensions are captured when paging starts; resize signals are not handled. Redirected output is written directly without alternate-screen controls.

## Library

Add the workspace package as a Git dependency:

```toml
scrl = { git = "https://github.com/laputa-systems/diff-pretty", package = "scrl" }
```

`DocumentBuilder` parses UTF-8 chunks with plain text and SGR styling.
`Document::line_text` returns visible text without line endings or SGR bytes;
logical lines include the empty line after a final newline.

```rust
use scrl::{DocumentBuilder, Session, SessionOptions, Size};

let mut builder = DocumentBuilder::new();
builder.push_str("plain\n\x1b[31mred\x1b[0m\n");
let document = builder.finish();
assert_eq!(document.line_text(1), Some("red"));

let mut session = Session::new(
    Size { rows: 24, columns: 80 },
    SessionOptions::default(),
)
.expect("valid session options");
session.push_chunk("one\ntwo\n");
session.finish();
```

`SessionOptions` supplies the title, optional search history, wrapping, follow
mode, and line filter. `Session::new` returns `io::Result` and reports an
invalid filter as `InvalidInput`; `run_reader`, `run_source`, and `run_document`
validate filters even when output is redirected. `RunOptions` pairs session
options with `PagingMode`. `ReaderSource`,
`FileSource`, and `FilesSource` implement `ChunkSource`; `run_reader` and
`run_source` run those sources, while `run_document` displays an existing
document. `Session` exposes `handle`, `advance`, `draw`, and `document` for
callers that provide their own input loop and terminal. Licensed under the
repository's [MIT license](../LICENSE).
