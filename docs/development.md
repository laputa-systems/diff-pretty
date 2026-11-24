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

## Coverage and mutation checks

Run `make coverage` to write a workspace HTML report and print the branch
summary. Coverage does not establish that assertions detect defects.

Run `make mutants-gate-test` to check the mutation gate. Run
`make mutants-diff BASE=origin/main SHARD=render` to check a component against
the merge base. The available shards are `cli`, `config`, `render`, `source`,
`session` and `other`. The gate requires Python 3.11 or newer and cargo-mutants.
Keep mutation build targets separate from ordinary test targets. A missing
result, unresolved timeout or exhausted deadline is not a passing result.

## Benchmarks

`make bench` runs the Edit tier (driver self-tests). `make bench-list` lists
all workloads, tiers and commands; `make bench-check BENCH_ARGS="--tier change"`
checks selected prerequisites without running them. Select a tier ceiling with
`BENCH_ARGS="--tier change"`, and repeat `--name NAME`, `--area render` or
`--area pager` or `--area cli`, and `--skip NAME` to narrow it. Names and areas are distinct.
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

The CLI workloads build the release binary in preflight. `cli_render_show_010`
measures a redirected process and checks every output byte against `render`;
`cli_pager_show_010` measures opening, paging, searching and quitting a PTY,
checking fixture-derived frame markers and restored terminal modes. Timings
include child startup and I/O. Allocator counts cover only the benchmark process,
not its child; child CPU and peak RSS are not measured.
