# Development

Run the common checks from the repository root:

```sh
make build
make test
cargo check --workspace --all-targets
```

`make test` runs workspace tests in release mode. `make test-ci` is the quiet release-mode test command used by the release workflow.

## Benchmarks

`make bench` runs the curated root suite through rustybench and saves a local,
machine-specific baseline. By default, the Makefile looks for rustybench in
the sibling `../rustybench` checkout; set `RUSTYBENCH` to override it. To
compare a candidate file, set `AFTER` to its path when running
`make bench-diff`.

The root suite covers rendering, alignment, allocation behavior, streaming,
retained documents, and pager viewports. The `scrl` suite is in
`scrl/benches/bench.rs` and can be run with:

```sh
cargo bench -p scrl --bench bench
```

See [architecture](architecture.md) for source locations and [release](release.md) for artifact builds.
