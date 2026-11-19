# diff-pretty

Rust renderer for Git-style unified diffs with an optional built-in pager.

## Commands

- Build: `make build`
- Test: `make test`
- Bench: `make bench` (Edit); `make bench-list`, `make bench-check` and `make bench-baseline`
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
- Edit: `cargo test --workspace --test cli`; twelve bounded process scenarios assert literal output, paging flags, help, operands, invalid UTF-8, terminal-control filtering and read/write failure exits. Current cost remains unqualified.
- Change: `make test` covers workspace unit cases, alignment API, CLI and PTY boundaries plus a doctest; no required missing-tool skip or ignored test was found. Ground's 1.40s warm cost above is the pre-change baseline, not a paired after-cost result.
- Scenario: the complete CLI selection is 22 caught/two unviable; numeric layout is 37 caught/one confirmed hang. Both have reconciled identities. The older whole-workspace 1,146-case run is incomplete; broader mutation and coverage remain open. Runs have a 1,200-second ceiling; larger matrices remain manual Full work.
- `diff-pretty` CLI paging flags, exact rendered output and exit 0/1/2: `tests/cli.rs`.
- Pager entry: `tests/pager_entry.rs` covers real PTY Auto, Always, Never and read error; `src/source.rs` tests cover fragmented chunks, public nonpaging input errors and literal output/write/flush boundaries. The complete adapter selection has seven caught mutants; latest-head Linux execution is checked separately.
- `scrl` CLI flags, help, operands, output and errors: `scrl/tests/cli.rs`; auto paging with an actual terminal: `scrl/tests/auto_paging.rs`; non-Unicode parsing: `scrl/src/main.rs`.
- Diff input and ANSI output: `src/render.rs` tests, including `public_render_to_matches_byte_exact_hunk_oracle`; full format coverage: none.
- Renderer API: `src/render.rs` tests; alignment API: `tests/alignment_api.rs`; remaining exported API: none.
- Numeric layout API: `src/config.rs` literal cell, append and decimal-boundary tests; both hand faults fail their intended assertions.
- Terminal controls: literal direct/reader/incremental API cases and an owned-child CLI oracle. The current 79-case selection has 70 caught, eight equivalent misses, one unviable and no timeouts; no exclusions. See `docs/deslop-cli-tests.md` for scope and dispositions. Quiet-host acceptance remains open.
- `scrl` library document, search, source and session behavior: tests in `scrl/src/`; session navigation frames/actions and complete 44-mutant five-method selection: [proof](docs/deslop-cli-tests.md#session-navigation). Live-loop, draw and full API coverage remain open.
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
