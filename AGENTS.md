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

## Topics

- Supported and unsupported input: [docs/input.md](docs/input.md)
- Output layout and ANSI rules: [docs/output.md](docs/output.md)
- Renderer and pager integration: [docs/pager.md](docs/pager.md)
- Package boundaries and source map: [docs/architecture.md](docs/architecture.md)
- Development, tests, and benchmarks: [docs/development.md](docs/development.md)
- Release builds and artifacts: [docs/release.md](docs/release.md)
