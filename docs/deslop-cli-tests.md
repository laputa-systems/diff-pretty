# CLI boundary qualification

The resumed Tests checkpoint is `5834fe7`, merged with current main at
`9fa3091`. This retains the full-help change. `674f456` replaces one existing
test's `.err().expect()` with `expect_err`; shipped runtime code is unchanged.

The current CLI suites pass six renderer and five pager process tests. They
use literal output and status assertions, bounded owned children and isolated
file fixtures. Six hand faults fail the intended assertions:

| Fault | Detecting boundary |
| --- | --- |
| Renderer reports success after I/O failure | Directory stdin and closed output pipe require exit 1 |
| Renderer prefixes redirected output | Every paging flag must produce the same literal rendered diff |
| Pager reports success after I/O failure | Invalid UTF-8, missing input and closed output pipe require exit 1 |
| Pager prints the wrong help command | Help bytes must match without reading invalid stdin |
| Pager reads only the first file operand | Two files must concatenate to literal `first\nsecond\n` |
| Pager prefixes redirected input | All noninteractive option combinations must preserve the input bytes |

Each fault produced its named assertion failure with Cargo exit 101, and
restoring the source returned the CLI suites to green. The first proof driver
expected verbose test output while Cargo printed terse output; that receipt
was rejected. The corrected reader requires each named failure section and its
assertion. No assertion or test was weakened to accept the receipt.

`make test`, `make lint`, and explicit workspace-wide clippy pass with one build
and test worker under `cpu-guard --cores 1`. `make test` uses the repository's
release profile. Workspace clippy found the pre-existing `err_expect` warning
in a pager test; its equivalent `expect_err` assertion passes its focused test.
These correctness runs do not establish quiet-host cost or performance.

The next Scenario selects all 24 mutants in the two CLI entry points: 12 each
in `src/main.rs` and `scrl/src/main.rs`. The inventory command is:

```sh
cargo mutants --workspace --list --json --file src/main.rs --file scrl/src/main.rs
```

Run a fresh baseline and the full workspace tests for this selection, with an
explicit test/build timeout, one mutation worker, a twenty-minute total budget
and the shared campaign lock. This Scenario is not yet qualified. Coverage,
quiet-host test cost and the broader renderer/pager mutation scopes remain open.
The old whole-workspace receipt remains incomplete: 109 untested mutants and
48 unconfirmed timeouts are not passes. DSLP-4 remains open.
