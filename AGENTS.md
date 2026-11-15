# diff-pretty

Rust renderer for Git-style unified diffs with an optional built-in pager.

## Commands

- Build: `make build`
- Test: `make test`
- Check: `cargo check --workspace --all-targets`

## Invariants

- Rendering is deterministic, uses fixed ANSI styles, and does not read terminal dimensions or user configuration.
- The renderer preserves ordinary leading metadata, removes its terminal controls, and restyles parsed diff content.
- Diff pager chunks keep each file together so word-diff pairing has complete file context.
- `scrl` remains independent of the renderer; redirected output never enters the pager or emits screen controls.
- Neither executable reads `$PAGER` or invokes an external pager.

## Verification

- `cpu-guard --cores 6 -- make test`: release workspace suite; 15.41s fresh target, 1.40s warm on macOS.
- `cpu-guard --cores 6 -- make lint`: all targets and features; 3.45s with a fresh test target.
- `diff-pretty` CLI invalid paging and exit 2: `tests/cli.rs`; valid flags, exit 0/1 and CLI output: none.
- `scrl` CLI auto paging output: `scrl/tests/auto_paging.rs`; non-Unicode operand parsing: `scrl/src/main.rs`; other flags and exit codes: none.
- Diff input and ANSI output: `src/render.rs` tests, including `public_render_to_matches_byte_exact_hunk_oracle`; full format coverage: none.
- Renderer API: `src/render.rs` tests; alignment API: `tests/alignment_api.rs`; remaining exported API: none.
- `scrl` library document, search, source and session behavior: tests in `scrl/src/`; full API coverage: none.
- Library consumers: `diff-pretty` uses `scrl`; `tests/alignment_api.rs` and root benchmarks use `diff-pretty`. External consumers are unknown.
- Perf workload: `make bench` (`benches/bench.rs`, including `render_show_large` and streaming); large diffs exercise the user-visible rendering hot path. It requires sibling `rustybench` or `RUSTYBENCH` override.
- CI suite has no missing-tool skip found; OS-gated terminal tests run on their matching Linux/macOS runner. No vendored trees found.

## Topics

- Supported and unsupported input: [docs/input.md](docs/input.md)
- Output layout and ANSI rules: [docs/output.md](docs/output.md)
- Renderer and pager integration: [docs/pager.md](docs/pager.md)
- Package boundaries and source map: [docs/architecture.md](docs/architecture.md)
- Development, tests, and benchmarks: [docs/development.md](docs/development.md)
- Release builds and artifacts: [docs/release.md](docs/release.md)
