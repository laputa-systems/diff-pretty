# Development

Run the common checks from the repository root:

```sh
make build
make test
cargo check --workspace --all-targets
```

`make test` runs workspace tests in release mode. The macOS release job runs
the quiet form, `make test-ci`. CI runs `cargo check --workspace --all-targets`
and `make test` on pushes and pull requests for Linux and macOS.

## Benchmarks

`make bench` runs the Edit tier (driver self-tests). `make bench-list` lists
all workloads, tiers and commands; `make bench-check BENCH_ARGS="--tier change"`
checks selected prerequisites without running them. Select a tier ceiling with
`BENCH_ARGS="--tier change"`, and repeat `--name NAME`, `--area render` or
`--area pager`, and `--skip NAME` to narrow it. Names and areas are distinct.
`--dry-run` prints commands without writing a run. Unknown or empty selections
fail. Full requires explicit `--tier full`; do not run it automatically.

Edit is estimated seconds; individual Change operations are estimated minutes
including compilation, and Scenario and Full may take longer. Rust and the
checked-in patch corpus are required for benchmark operations. Each run writes
logs and a summary under `.work/bench/` with exclusions and host information.
The short single-sample runs establish completion, not a performance verdict;
they are not host-gated. The original measurements and sample defaults remain
available via `make bench-baseline`: it runs the root suite through rustybench
and persists `benches/baseline.json`. The default sibling `../rustybench`
checkout may be replaced with `RUSTYBENCH=rustybench`. Compare a candidate with
`make bench-diff AFTER=path/to/baseline.json` using the same override.

The root suite covers fixture render bundles, retained documents, streaming,
color, plain diffs and viewport writes. The `scrl` suite covers synthetic
redraw and search plus filtered real-fixture streams at 50 KiB, 500 KiB and
5 MiB. Setup is outside the measured closures except where the operation is
construction. For profiling, use platform CPU sampling and allocation tools in
separate runs; `rustybench syscalls` offers Linux syscall diagnostics, not timing.
No current automatic comparison qualifies host load or interleaves revisions.

See [architecture](architecture.md) for source locations and [release](release.md) for artifact builds.
