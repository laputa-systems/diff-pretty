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

## Terminal-control rendering

The next component adds independent literal output for direct, reader and
incremental public rendering, plus a bounded real-executable oracle. Cases
cover CSI, OSC with BEL or ST, DCS/PM/APC/SOS, embedded controls, malformed and
incomplete sequences, Unicode and tabs. Plain unified input also preserves
its literal path decoration. Disabling string-control termination passes all
six previous CLI cases but fails the new oracle; metadata filtering, hunk
filtering and the plain-input flag each have a restored fault witness.

The expanded CLI case fails on unmutated `7711c9a`: a plain, uncolored hunk
retains NUL and C1 controls. `render_chunk` enabled stripping only when the
input contained ESC. Fix `f17f82f` shares `has_terminal_controls` with the
stripper and checks the parsed lines, retaining tabs and ordinary Unicode.
The same byte oracle passes after the fix. Public APIs, options and dependencies
are unchanged; this repairs the documented terminal-control contract.

| Selection | Caught | Missed | Unviable | Timeout |
| --- | ---: | ---: | ---: | ---: |
| `49a1c97`, original three sanitizer functions | 4 | 74 | 1 | 0 |
| `7711c9a`, same 79 identities with initial new tests | 57 | 21 | 1 | 0 |
| `f17f82f`, shared predicate and expanded oracle | 70 | 8 | 1 | 0 |

The final inventory still has 79 entries, but includes the new predicate and
replaces the former fast-path expressions; its identities are not the same
selection as the first two rows. Each inventory also contains a tool-selected
`FileInfo::new_plain` field mutation outside the function filter. That mutation
is caught by the new literal plain-input case. No generated entry was dropped.

The eight final misses are equivalent for observable output:

- `has_terminal_controls` returning true, or changing its `&&` to `||`,
  disables borrowing without changing the bytes the stripper produces.
- Both `push_metadata_line` increments in its `None` fallback are unreachable:
  the caller has established ESC, for which the scanner always returns `Some`.
- Three scanner mutations change only the SGR-classification Boolean. Both
  callers ignore that field; sequence consumption is unchanged.
- Multiplying rather than adding one at the OSC BEL terminator leaves BEL
  for the caller's next iteration, where it is removed as a bare control.

The unviable `&&` to `||` replacement would introduce an invalid `let` chain.
Every caught log contains a failure of the new CLI oracle. Its existing owned
child helper kills and reaps a nonterminating executable after five seconds;
those hangs are caught assertion failures, not cargo-mutants Timeout results.
Raw tool exit 2 remains visible for the equivalent misses. No exclusion was added.

Reproduce the final Scenario with cargo-mutants 27.1.0, one build job and test
thread, under the shared mutation lock and a 1,200-second total bound:

```sh
cpu-guard --cores 2 -- cargo mutants --gitignore true --workspace \
  --file src/render.rs \
  --re 'has_terminal_controls|strip_terminal_controls_append|push_metadata_line|terminal_control_end' \
  --test-workspace true --cargo-test-arg=--test --cargo-test-arg=cli \
  --baseline run --timeout 60 --build-timeout 300 --jobs 1 --no-shuffle \
  --output .work/controls-final
```

The actual test phase runs the renderer package's `cli` target: six tests
before, seven after. Full ordinary verification remains `make test`, `make lint`
and warnings-denied workspace clippy, all passing with a separate final target.
Mutation and ordinary build targets must remain separate.

The macOS ARM64 release-profile binaries both occupy 855,480 bytes. Their hashes
differ, and executing them confirms the old binary retains a bare NUL while the
candidate removes it. Five alternating RSS observations per side for each of
`log_000.patch` and `log_000_color.patch` produce identical per-fixture output
hashes and twenty successful exits. RSS ranges in bytes are respectively
3,047,424–3,162,112 before versus 3,014,656–3,047,424 after, and
3,112,960–3,145,728 before versus 3,047,424–3,080,192 after. These are bounded
memory observations, not a claim of reduced memory use or production limits.
Perf gate deferred: host busy. The declared workload's five paired timing runs,
test-cost acceptance, coverage and other decision regions remain open; the
draft stays unmerged.

## Current workspace branch coverage

At runtime revision `de831c1264a464784e484267f3604dea04667ce9`, macOS ARM64,
the pinned nightly and cargo-llvm-cov 0.9.1, the complete workspace coverage
run passes all 70 tests with no ignored cases. Default features and debug
instrumentation cover 3,246/4,308 lines and 593/924 branches across the 13
executable first-party source files. The historical before result was
3,025/4,182 lines and 542/918 branches; changed denominators prevent treating
the difference as a fixed-scope detection score. Coverage is a report, not
an acceptance threshold or proof that an assertion detects a defect.

The report leaves these decisions open:

- `src/source.rs`: `emit_reader` and `DiffSource::produce` are unexercised.
  Redirected CLI tests take main's direct rendering branch, so they do not
  witness this public adapter or the live pager. Add an owned PTY scenario
  and an adapter-level output/error oracle; do not infer a pass from the
  renderer tests.
- `scrl/src/session.rs` and `scrl/src/terminal.rs`: live session, event and
  terminal lifecycle paths remain partly unrun. DSLP-8's retained-source
  runner is a liveness investigation, not permission to label these paths
  unreachable or remove them.
- `src/render.rs`: metadata/hunk rendering decisions outside the qualified
  sanitizer remain open. Within the sanitizer, the non-ESC scanner entry
  and metadata `None` fallback are guarded by their callers, as the mutation
  equivalence arguments above explain.
- Search, document editing and streaming-source error decisions in `scrl`
  still need individual witnesses or dispositions. This native report does
  not qualify Linux-specific terminal behavior or the old Full survivors.

Reproduce under the shared mutation/coverage lock, with
`PATH=/Users/josh/d/stuff/.work/tools/bin:$PATH`, `TMPDIR=$PWD/.work/tmp`,
`RUSTC_WRAPPER=kache`, `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1` and
`CARGO_TARGET_DIR=$PWD/.work/target-coverage-controls`:

```sh
cpu-guard --cores 2 -- cargo llvm-cov --workspace --branch --json \
  --output-path .work/tmp/coverage-controls.json
cpu-guard --cores 2 -- cargo llvm-cov report --workspace --branch --summary-only
```

Both commands exit 0. Keep `--workspace` on the report command too: omitting
it reports only the renderer package, not this result. Timing/test-cost and
performance gates remain deferred on the busy host; DSLP-4 stays open.

## Pager entry and source adapter

At base `7b2a67e2eb7e56ad27cdcf2877f375bd3303e549`, macOS PTY cases
exercise Auto short output, Always entry and quit after an observed payload,
Never, and an Auto input read failure. The PTY master is nonblocking; an owned
child is killed and reaped on panic, with a five-second deadline. The tests
check literal output, terminal modes, screen exit, status and stderr. Source
unit cases use two-byte buffering, literal emitted chunks, and read and callback
errors. The callback error stops further emission. Forcing the session paging
field to Auto made the Always test fail at its observed-payload assertion; the
fault was restored.

Complete `src/source.rs` cargo-mutants 27.1.0 selection (no config or skipped
calls) has six generated mutations, all caught, with no misses, timeouts or
unviable mutations. The fresh baseline ran the root package suite, including
20 library tests and four PTY tests. Every caught log was inspected: true and
`&&` to `||` fail the public policy test; false and inverted fallback fail the
Always PTY test; empty `emit_reader` fails Auto and Always PTY cases; empty
`produce` fails both source unit cases. The public policy test compares all
three modes against actual stdout terminal state. The CLI tests exercise modes
on a PTY and redirected output. `emit_reader`'s own redirected public fallback
remains unwitnessed because `main` routes redirected output around it. No public
test fixture, global stdio mutation, or libtest subprocess parsing was introduced.

Reproduce with the shared `mutation.lock` acquired by atomic `mkdir` and
released by a shell EXIT trap, `PATH=/Users/josh/d/stuff/.work/tools/bin:$PATH`,
`TMPDIR=$PWD/.work/tmp`, `RUSTC_WRAPPER=kache`, `CARGO_BUILD_JOBS=1`,
`RUST_TEST_THREADS=1`, and a mutation-specific `CARGO_TARGET_DIR` distinct
from ordinary tests:

```sh
timeout -k 5 1200 cpu-guard --cores 2 -- cargo mutants --in-place --no-config --skip-calls-defaults=false -f src/source.rs -p diff-pretty --test-package diff-pretty --timeout 60 --build-timeout 300 --output .work/tmp/pager-mutation-policy -- --offline
```

The native PTY witnesses have only been run on macOS; Linux, other exported
APIs, broader mutation and coverage, and quiet-host test cost/performance
remain open. The old 1,146-mutant Full run remains incomplete, with 109
untested and 48 unconfirmed timeouts. Existing CLI, numeric and terminal
control selection counts are unchanged. DSLP-4 remains open.
