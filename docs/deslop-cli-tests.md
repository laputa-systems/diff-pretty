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
restoring the source returned the CLI suites to green.

`make test`, `make lint`, and explicit workspace-wide clippy pass with one build
and test worker under `cpu-guard --cores 1`. `make test` uses the repository's
release profile. Workspace clippy found the pre-existing `err_expect` warning
in a pager test; its equivalent `expect_err` assertion passes its focused test.
These correctness runs do not establish quiet-host cost or performance.

The Scenario selects all 24 mutants in the two CLI entry points: 12 each
in `src/main.rs` and `scrl/src/main.rs`. The inventory command is:

```sh
cargo mutants --workspace --list --json --file src/main.rs --file scrl/src/main.rs
```

The fresh baseline and full workspace tests ran under cargo-mutants 27.1.0,
one mutation worker, the shared campaign lock and a twenty-minute total bound:

```sh
cargo mutants --gitignore true --workspace --file src/main.rs --file scrl/src/main.rs \
  --test-workspace true --baseline run --timeout 60 --build-timeout 300 \
  --jobs 1 --no-shuffle --output .work/cli-scenario
```

Both runs used default mutation test/build profiles, one Cargo build job,
`RUST_TEST_THREADS=1` and `cpu-guard --cores 1` on macOS ARM64.

| Candidate | Caught | Missed | Unviable | Timeout |
| --- | ---: | ---: | ---: | ---: |
| `1e9ee30` | 21 | 1 | 2 | 0 |
| `ac5ca97` | 22 | 0 | 2 | 0 |

The missed inversion of the `--no-pager` comparison is observable only when
pager choice matters; redirected process output alone cannot distinguish it.
A literal parser-contract table now guards all supported modes and the default.
The same inverted comparison fails that new assertion with actual `Always`
instead of required `Never`. The two unviable default-return replacements fail
compilation because `PagingMode` and `ParsedArgs` do not implement `Default`.
All 24 identities match the inventory, and every caught log reports an affected
CLI/parser, help, file or I/O contract failure. No exclusions were added.

This qualifies the native CLI component's defect detection. Coverage,
quiet-host test cost and the broader renderer/pager mutation scopes remain open.
The old whole-workspace receipt remains incomplete: 109 untested mutants and
48 unconfirmed timeouts are not passes. DSLP-4 remains open.

## Numeric layout boundary

The next bounded component is the exported `config` API. Literal cell strings
cover missing numbers, zero width, parity, overflow width and append behavior.
Decimal digit counts cover zero and both sides of every representable power
of ten through 10^19. The expected strings and digit counts are independent
of the functions under test.

Two hand faults fail the intended assertions: replacing centered left padding
with zero, and changing the 10,000 loop boundary from inclusive to exclusive.
Both faults were restored. Focused tests, the complete release workspace suite
and warnings-denied workspace clippy pass. Current source changes are test-only.
The production logic is unchanged from the saved before selection. All 38 numeric
identities reconcile: that region of the older, incomplete Full run recorded
17 caught, 20 missed and one unresolved timeout. The fresh bounded Scenario
at `10e3782` records 37 caught, zero missed or unviable, and one timeout.
Every caught log contains a new numeric-boundary assertion failure.

The timeout flips `num_digits`' loop from `n >= 10_000` to `n < 10_000`.
For input 1, division reaches zero and the condition stays true. A fresh
baseline passes; the exact mutant still hangs in the new digit-count test
with a 120-second timeout, twice the first run's 60 seconds. It is a confirmed
hang detection, not an unresolved timeout or an ordinary caught count. Both
tool receipts return exit 3 because the tool retains the Timeout class.

Both runs use cargo-mutants 27.1.0, two guarded cores, one build job and test
thread, one mutation worker, a 300-second build timeout and a 1,200-second
total bound. Reproduce the complete selection with:

```sh
cargo mutants --gitignore true --workspace --file src/config.rs \
  --test-workspace true --baseline run --timeout 60 --build-timeout 300 \
  --jobs 1 --no-shuffle --output .work/config-scenario
```

Repeat the one timeout with `--timeout 120` and
`--re '^src/config[.]rs:80:13: replace >= with < in num_digits$'` in a separate
output directory. No exclusion was added. This qualifies the numeric component
with 37 caught and one confirmed hang; the broader renderer/pager mutation,
coverage, quiet-host cost and whole-change CI lane remain open.
