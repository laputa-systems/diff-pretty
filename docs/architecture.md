# Architecture

The workspace has two packages. The root `diff-pretty` package parses and renders patches; `scrl` is a generic pager library and standalone command. The dependency runs from `diff-pretty` to `scrl`, never the reverse.

## Renderer

- `src/lib.rs` exports the render API and the `WIDTH` constant.
- `src/render.rs` parses patches, styles output, and provides string, writer, reader, and retained-document entry points.
- `src/config.rs` defines ANSI styles and number formatting.
- `src/align.rs` implements sequence alignment and exposes its token sequences read-only so cached tables remain consistent; `src/edits.rs` pairs removed and added lines and infers word changes.
- `src/source.rs` adapts rendered chunks to `scrl`.

## Pager

- `scrl/src/document.rs` stores visible text and composes viewports.
- `scrl/src/search.rs` matches visible text.
- `scrl/src/session.rs` owns terminal-independent navigation state and connects sources to the session.
- `scrl/src/source.rs` provides reader and file sources.
- `scrl/src/terminal.rs` handles Unix terminal mode, signals, screen controls, and key decoding.
- `scrl/src/main.rs` is the standalone pager command.

The root CLI is in `src/main.rs`. Benchmarks use `benches/bench.rs` and `scrl/benches/bench.rs`; representative patch inputs are under `fixtures/`.
