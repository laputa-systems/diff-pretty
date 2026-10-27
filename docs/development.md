# Development

Run the common checks from the repository root:

```sh
make build
make test
cargo check --workspace --all-targets
```

`make test` runs workspace tests in release mode. The macOS release job runs
the quiet form, `make test-ci`. CI runs `cargo check --workspace --all-targets`
and `cargo test --workspace` on pushes and pull requests for Linux and macOS.

## Benchmarks

`make bench` runs the curated root suite through rustybench and saves a local,
machine-specific baseline. By default, the Makefile looks for rustybench in
the sibling `../rustybench` checkout; set `RUSTYBENCH` to override it. To
compare a candidate file, set `AFTER` to its path when running
`make bench-diff`.

The root suite covers rendering, alignment, allocation behavior, streaming,
retained documents, and pager viewports. Its show-fixture bundles are the
named lists in `benches/bench.rs`, not a size cutoff. The `scrl` suite is in
`scrl/benches/bench.rs` and can be run with:

```sh
cargo bench -p scrl --bench bench
```

See [architecture](architecture.md) for source locations and [release](release.md) for artifact builds.
