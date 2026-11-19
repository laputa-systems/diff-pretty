Prefix: DSLP
Next: 12

Deslop campaign backlog ([plan](https://github.com/laputa-systems/stuff/blob/main/docs/campaigns/deslop.md), tier A). One agent works this file with the `deslop` skill: take the lowest-numbered open ticket whose area does not start with `Needs owner ·`. After DSLP-5, work the open P1 and P2 findings, then fill in `## Report` below.

## Report

Bench audit: root show typical/metadata/large/corpus measure render throughput;
log, color and plain measure format paths; reader-to-sink measures streaming;
retained document and synthetic log measure document construction; viewport
measures a retained frame. The scrl synthetic redraw, clipping and search cases
measure those operations; 50 KiB/500 KiB/5 MiB filtered real-fixture streams
measure session construction and redraw, with line-count and nonempty-frame
checks. Most other cases black-box output or length without an output oracle.
Allocation counts and steady-state elapsed time exclude setup and CLI startup;
no process-tree CPU, peak RSS, I/O or live-terminal journey is measured. Ranked
DSLP-10 (P2) covers user-facing journey and correctness gaps; DSLP-11 (P3)
covers attribution and comparison validity. CPU sampling, allocation profiling
and Linux `rustybench syscalls` are separate, overhead-bearing diagnostics.
The saved host baseline is a comparison anchor, not an interleaved host gate;
no budget automatically detects a regression. Fixtures are checkout files, not
separately pinned; the old sibling rustybench prerequisite is checkout-dependent.
No open Bench ticket existed. Full was not run.


DSLP-5: `run_source` handles `Never` before terminal dispatch; `Auto` and
`Always` enter the live runner. The unreachable retained-source branch and
its only `read_event_tty` adapter were removed (DSLP-8). The separate
`run_document` retained runner remains live through its public entry point.
No package, script, documentation or test registers the removed path. The
branch was Unix terminal-feature code; the non-Unix and feature-disabled
fallbacks are unchanged. Workspace CLI and PTY tests cover the consumers.

The only `allow(dead_code)` was on `KeyDecoder::timed_out`, called by
`read_event_inner` for idle TTY reads and by a decoder test; the unnecessary
suppression was removed. The test-only `DiffRenderer::document` is used by
renderer tests, `read_event` by terminal tests, and the other `cfg(test)`
blocks register tests; their local helpers are used by those tests. Public
library entry points remain available to unknown external consumers.

`cargo-machete` identified the root's unused `regex-lite`; only `scrl`
imports it, so the root declaration was removed and scrl's kept. The rustc
scan identified unreachable `pub` on two integration-test support functions;
`tests/cli.rs` and `scrl/tests/cli.rs` call `support::run`, and
`tests/cli.rs` calls `bare_controls::run`, so their visibility is now
`pub(crate)`. A no-default-features scan warns on `Document::filtered`,
`SourcePull`, and `source_pull_request`: the terminal-feature retained runner
calls `Session::from_document` and `filtered`, while the live runner calls
`source_pull_request` and uses `SourcePull`. They are live with terminal
support and the latter two also have session tests.

On macOS arm64, the locked workspace all-target rustc scan is clean after
these changes; cargo-machete 0.10.1 reports no unused dependencies. The
release workspace suite and all-feature lint pass. The no-default-features
check passes with the three conditional-use warnings above. Linux execution
and external consumers were not tested. No public contract was changed.


DSLP-4 closed under the revised 2026-11-19 mutation policy. This disposition
supersedes the earlier open-status statements below, which record past results.

- Rendered diff bytes, metadata and terminal-control filtering: `tests/cli.rs`
  and `src/render.rs` literal public-renderer oracles; streaming and error
  propagation: `src/source.rs` and `scrl/tests/source_contracts.rs`.
- Pager frames, navigation, search and input lifetime: `scrl/src/session.rs`
  literal draw/navigation/search tests and `scrl/tests/source_contracts.rs`;
  live entry, quit and output: `tests/pager_entry.rs` real PTY cases. The
  draw fault and selection are documented in [draw proof](docs/deslop-session-draw.md).
- Terminal restoration and signal exit: `tests/pager_entry.rs` real PTY quit
  and SIGTERM cases; `scrl/src/terminal.rs` raw-mode and signal-guard tests.
- Exit codes and I/O errors: `tests/cli.rs`, `scrl/tests/cli.rs` and the pager
  PTY read-error case exercise success, invalid input and read/write failures.

The retained required changed-source and protected-region gate is recorded in
#31, with negative controls #36–39; its configured critical scopes cover CLI,
numeric layout, terminal-control filtering, source, session navigation and
draw. Wider renderer and aggregate live-session inventories are advisory, not
closure gates. Their incomplete outcomes and unconfirmed timeouts are not
passes. The PTY cases protect entry, quit and signal cleanup, not every live
loop interleaving or resize; full renderer format and external consumer coverage
remain unverified. No new mutation trial was needed: 0 runner-minutes spent in
this closeout. The earlier five warm pairs had equal 1.19s medians for their
identified revisions; no current paired cost or quiet-host performance claim is
made. Coverage is a report, not a threshold. The coverage target from 013c089
was already integrated as #41 (b8a47c9).

PR [#31](https://github.com/laputa-systems/diff-pretty/pull/31) qualified
the changed-source and protected-region gate on macOS arm64 and Linux arm64.
The 12 successful mutation jobs used 2,276 runner seconds in aggregate.
The mixed change, deletion, uncovered branch and documentation controls
are recorded in #36–39. On macOS arm64, five interleaved warm suite pairs
against `49a1c97` had a median of 1.19 seconds for each revision and ran
all 68 baseline and 89 candidate tests. The #31 receipt identifies the
workload-qualified runtime inputs. Remaining renderer and live terminal
coverage keep DSLP-4 open.

Session navigation: three literal frame/action scenarios cover page and edge
movement, horizontal shifts, follow, help and search routing. The complete
44-mutant five-method selection has 43 caught and one unviable, with no misses
or timeouts; a restored page-down fault fails the new frame oracle. See
[session navigation proof](docs/deslop-cli-tests.md#session-navigation).
Session has no resize or half-page event/API; different initial sizes are tested.
Live-loop and draw coverage, broader DSLP-4 and quiet-host gates remain open.

Streaming-source completion: the complete ten-identity selection now has nine
caught and one unviable, with no misses, timeouts or exclusions. The two
initial empty-callback survivors fail the new rejecting consumer. Bounded
semantic child assertions cover UTF-8, ordered/lazy files and error priority;
full workspace tests, lint and clippy pass. Broader coverage and gates stay open.

Pager-entry and source-adapter completion: four real PTY cases and literal
stream/error oracles pass. The complete current adapter inventory has seven
caught, no other outcomes or exclusions, including the private production
render/flush boundary. Restored dropped-error and skipped-flush faults fail.
The release workspace suite, lint and workspace clippy pass (79 executable
cases and one doctest). Broader coverage/API and quiet-host gates remain open.

Pager-entry continuation: owned PTY and source-stream tests now witness Auto,
Always, Never and read errors on macOS. The complete six-mutant source
selection has six caught and no misses; the redirected public `emit_reader`
fallback remains untested. Linux and the
quiet-host performance gate remain open. See [pager entry proof](docs/deslop-cli-tests.md#pager-entry-and-source-adapter).

Current native workspace coverage at runtime `de831c1` passes all 70 tests:
3,246/4,308 lines and 593/924 branches. The public pager adapter, live session
and terminal decisions, other renderer regions and remaining streaming/error
branches still need witnesses or dispositions. This does not qualify the old
Full survivors, Linux behavior or busy-host cost/performance gates; DSLP-4
remains open. Commands and limits: [coverage record](docs/deslop-cli-tests.md#current-workspace-branch-coverage).

Owner stop, 2026-11-16: the Tests continuation adds nine CLI process cases
across diff-pretty and scrl, with owned-child deadlines and literal output,
file, error and closed-pipe contracts. Those cases and `make lint` passed.
Hand faults, final full checks, paired test cost and the mutation gate remain
unverified; DSLP-4 stays open. The before coverage is 3025/4182 lines and
542/918 branches. The 1200-second workspace mutation run exhausted its budget:
1037 of 1146 completed (497 caught, 439 missed, 53 unviable, 48 unconfirmed
timeouts), leaving 109 untested. It is not a pass. Preserve
`.work/mutants-workspace-before/` and `.work/tmp/mutation-before.log`.
The current component selection needs narrowing before another bounded run.
The owner resumed on 2026-11-16. The branch now contains current main and the
full-help contract. Eleven current CLI cases, the full workspace suite and
workspace clippy pass. Six hand faults fail their intended assertions and are
restored. The complete 24-mutant CLI Scenario found one missed paging binding;
a literal parser-contract table catches it. The fresh result is 22 caught,
two compiler-unviable, zero missed and zero timeouts, with the same 24 identities.
[The CLI qualification record](docs/deslop-cli-tests.md) names each fault and
the remaining coverage, mutation and quiet-host cost gates. Numeric layout
now has literal alignment and decimal-boundary oracles, each proved by a hand
fault. Its complete 38-mutant selection records 37 caught and one hang confirmed
with a fresh baseline and a doubled timeout, zero missed or unviable. Every
caught log contains a new numeric assertion failure. The broader renderer/pager
scope and quiet-host acceptance remain open; DSLP-4 remains open.

Terminal-control continuation: literal API and process oracles catch a defect
the old CLI cases miss. Uncolored hunks previously retained bare NUL and C1
controls because parsing only stripped chunks containing ESC. `f17f82f` shares
the control predicate and fixes the demonstrated regression while retaining
tabs and ordinary Unicode. The final bounded selection records 70 caught,
eight equivalent misses, one unviable and zero timeouts, with complete
dispositions and no exclusions. Full release workspace tests and lint pass.
Release binary sizes match; paired RSS observations and qualification limits
are recorded in the CLI evidence. Perf gate deferred: host busy. The declared
workload, test-cost gate, other regions and whole-change CI lane remain open.

Ground base: `e081d90fb98020ba64b307da178ccf3068e7e73e`. Rust code: 6.0k lines (fleet/loc/v1, vendored included, seeded). On macOS with a fresh scratch target, guarded `make test` took 15.41s cold and 1.40s warm; guarded `make lint` took 3.45s. All passed. PR CI now calls `make test`; platform-specific terminal tests run on their matching OS, and no missing-tool skip was found. No vendored trees found. The existing large-diff rendering benchmark is the declared perf workload; later scorecard measurements are pending.

Types: escape-hatch lines 139 → 133. Candidate findings are deduplicated source locations; test and benchmark findings are excluded for unwrap/expect. Real means defect or deletable escape hatch; FP includes intentional code or findings outside typing scope.

| Setting | Findings | Real | FP | Decision |
| --- | ---: | ---: | ---: | --- |
| unsafe_code forbid | 11 | 0 | 11 | reject: terminal FFI and PTY test |
| wildcard_enum_match_arm | 3 | 0 | 3 | reject: intentional event fallbacks |
| match_wildcard_for_single_variants | 0 | 0 | 0 | reject: no witness |
| cast_possible_truncation | 0 | 0 | 0 | reject: no witness |
| cast_sign_loss | 0 | 0 | 0 | reject: no witness |
| unwrap_used | 4 | 0 | 4 | reject: bounded UTF-8 and nonempty array |
| expect_used | 22 | 6 | 16 | reject: invariant checks exceed FP threshold |
| redundant_clone | 2 | 0 | 2 | reject: not a typing defect |
| needless_pass_by_value | 0 | 0 | 0 | reject: no witness |

Types qualification: [evidence](docs/deslop-types.md). The private filtered
state witness rejects a missing source with E0308. Workspace tests and lint
pass. Five interleaved pairs for both renderer and pager suites, including
real filtered streaming at three scales, pass the host gates; no slowdown
exceeds both 2% and the observed spread. Reported allocation counters and
release binary sizes are unchanged; peak RSS ranges overlap. Public APIs,
formats and dependencies are unchanged. The workspace is the affected consumer.

The real PTY test now sends SIGTERM after the first interactive frame.
It checks the original signal exit, restored terminal modes and the
alternate-screen cleanup bytes. Against `b8a47c9`, two independent
controls made the test fail on the missing signal exit: replace the
body of `reraise_term` with an empty body, or make `terminated`
return `false`. Reproduce with
`cargo test --locked --test pager_entry sigterm_restores_terminal_and_preserves_signal_exit -- --exact`
in a disposable copy. The five pager entry cases and Clippy passed on
macOS ARM64. Broader terminal and live-loop mutation remain open.

The terminal fixture now checks exact input events for supported keys,
unsupported modified sequences, malformed UTF-8 followed by valid text,
and EOF without repeated reads. A real PTY checks raw mode restoration
and re-entry. A child process checks that the signal guard restores both
prior dispositions and consumes a suspension request once. A closed
output catches an early successful return from the suspend path.

On macOS ARM64, 22 of 23 targeted cargo-mutants 27.1.0 controls failed
the new checks, including the former 60- and 180-second EOF timeout.
The one survivor removes the ASCII dispatch arm in `KeyDecoder::next`.
The production reader feeds each byte before calling `next`, and the
UTF-8 fallback produces the same event for that individual ASCII byte.
This disposition covers the current reader. Wider live-loop mutation
and measured test cost remain open.

Reproduce in a disposable checkout using cargo-mutants 27.1.0 with
`--workspace --file scrl/src/terminal.rs --test-package scrl --cargo-test-arg=--lib`.
Select the affected match arms in `KeyDecoder::next`,
`escape_event` and `utf8_event`, the EOF guard in `read_event_inner`,
`RawMode::restore` and `reenter`, `SignalGuard::drop`,
`suspend_requested` and `suspend`. The ordinary workspace passed 94
tests and one doctest. Clippy passed.

**Session input lifetime**

Two public session cases verify final bytes after EOF and one-line
navigation after a filtered tail stops matching. They caught the two
selected mutations in `Session::finish` and `Session::push_chunk` with
cargo-mutants 27.1.0. All four source-contract cases and Clippy passed.
Select those mutations in `scrl/src/session.rs` and run the
`source_contracts` integration test.

The [unreachable-runner trial](https://github.com/laputa-systems/diff-pretty/tree/67ef81a)
passed the local terminal shard with 59 catches, ten compile-invalid
cases and one scoped equivalent. Its session shard accounted for all
154 cases with 100 catches, 17 compile-invalid cases and 37 survivors,
including the known draw-capacity equivalent. Run the original gate
against `c1c81e1` using `make mutants-diff BASE=c1c81e1 SHARD=other`
or `SHARD=session`. The new focused tests catch two of the session
survivors. The aggregate session gate still needs wider lifecycle,
search and live-loop work before the deletion can merge.

Two more session tests exercise search recentering and retain the
selected occurrence of a repeated line across filtered input. The 28
session unit tests and Clippy passed. At `9ce6235`, five injected
changes failed the two tests. These replace `advance` with false,
discard the saved match, invert range equality, or replace the nearest
distance comparison with equality or a greater-than comparison.

Run `cargo test --locked -p scrl --lib session::tests`, then rerun the
two named cases with each change applied separately to
`scrl/src/session.rs`. The less-than-or-equal change survives. The
sole caller saves the match after removing the stale tail and before
appending. The saved match remains at its original line with distance
zero, so a different equally close line cannot occur.

## Tickets











### DSLP-10 · P2 · Measure complete CLI rendering and pager journeys

- **Area:** Bench · user journeys
- **Where:** benches/bench.rs; scrl/benches/bench.rs
- **Problem:** The existing in-process benchmarks omit CLI startup, redirected I/O and live terminal paging, so user-visible latency, CPU and memory regressions can ship unseen. Most render operations only consume output length without a correctness oracle.
- **Recommendation:** Add representative end-to-end CLI and PTY workloads with output and completion checks, resource measurements and a comparable reference.
- **Fix with:** refactor-benchmarks
- **Effort:** L

### DSLP-6 · P3 · Guard CLI success and failure contracts at the process boundary

- **Area:** CLI · tests
- **Where:** tests/cli.rs:4; src/main.rs:20; scrl/src/main.rs:54; scrl/tests/auto_paging.rs:13
- **Problem:** Test-gap finding: the root CLI process test only rejects an invalid paging value, while valid paging flags, rendered output, successful exit, and I/O failure exit are not asserted through the executable. The scrl process test covers short auto paging but not its other flags or error exits. Regressions in argument wiring or status mapping can therefore pass the workspace suite.
- **Recommendation:** Add bounded process-level scenarios for valid direct output and representative invalid input and I/O errors for both commands, with literal output and exit assertions. Include src/main.rs and scrl/src/main.rs in the Tests step's critical mutation scope.
- **Fix with:** test-audit
- **Effort:** M


### DSLP-11 · P3 · Qualify benchmark comparisons and attribution

- **Area:** Bench · measurement
- **Where:** benches/bench.rs; docs/development.md
- **Problem:** Steady-state allocator counts and elapsed samples do not explain CPU, peak footprint or I/O, and the persisted host baseline is not an interleaved gated reference. Fixture and host changes can confound comparisons.
- **Recommendation:** Qualify interleaved comparisons and fixture identity, and document separate CPU sampling, allocation and syscall attribution runs with their overhead.
- **Fix with:** benchmark-checklist
- **Effort:** M
