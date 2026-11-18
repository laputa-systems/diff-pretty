Prefix: DSLP
Next: 10

Deslop campaign backlog ([plan](https://github.com/laputa-systems/stuff/blob/main/docs/campaigns/deslop.md), tier A). One agent works this file with the `deslop` skill: take the lowest-numbered open ticket whose area does not start with `Needs owner ·`. After DSLP-5, work the open P1 and P2 findings, then fill in `## Report` below.

## Report

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




### DSLP-4 · P2 · Tests: coverage, boundary gaps, mutation

- **Area:** deslop · tests
- **Where:** test suites and CI
- **Problem:** Workspace coverage and the retained protected-region mutation lane are measured. The wider renderer and live session inventory, paired cost and remaining outcomes are open.
- **Recommendation:** Run the deslop Tests step. Accept: no undispositioned survivor in critical code, the test cost budget holds, and the lane is green where the tools are mirrored.
- **Fix with:** deslop
- **Effort:** M

### DSLP-5 · P2 · Dead code: tools and debloat

- **Area:** deslop · dead code
- **Where:** whole repository
- **Problem:** No dead-code scan has run.
- **Recommendation:** Run the deslop Dead code step. Accept: tools clean or allowlisted with reasons; walk-backs filed as Needs owner tickets.
- **Fix with:** deslop
- **Effort:** M






### DSLP-6 · P3 · Guard CLI success and failure contracts at the process boundary

- **Area:** CLI · tests
- **Where:** tests/cli.rs:4; src/main.rs:20; scrl/src/main.rs:54; scrl/tests/auto_paging.rs:13
- **Problem:** Test-gap finding: the root CLI process test only rejects an invalid paging value, while valid paging flags, rendered output, successful exit, and I/O failure exit are not asserted through the executable. The scrl process test covers short auto paging but not its other flags or error exits. Regressions in argument wiring or status mapping can therefore pass the workspace suite.
- **Recommendation:** Add bounded process-level scenarios for valid direct output and representative invalid input and I/O errors for both commands, with literal output and exit assertions. Include src/main.rs and scrl/src/main.rs in the Tests step's critical mutation scope.
- **Fix with:** test-audit
- **Effort:** M


### DSLP-8 · P3 · Investigate unreachable legacy terminal runner branch

- **Area:** pager · dead code
- **Where:** scrl/src/lib.rs:60; scrl/src/session.rs:595; scrl/src/session.rs:608
- **Problem:** Dead-code candidate: run_source returns before calling run_terminal when paging is Never, and Always or Auto immediately dispatch to run_terminal_live. The subsequent retained-source branch appears unreachable through the sole production caller. Its liveness across tests, features, and platforms is unproven, and it duplicates terminal lifecycle logic.
- **Recommendation:** Trace all callers and supported feature and platform configurations; if no caller reaches the branch, remove it and its exclusively used helpers while preserving the public runner and testing its consumers.
- **Fix with:** dead-code-archaeology
- **Effort:** S
