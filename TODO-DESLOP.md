Prefix: DSLP
Next: 12

Deslop campaign backlog ([plan](https://github.com/laputa-systems/stuff/blob/main/docs/campaigns/deslop.md), tier A). One agent works this file with the `deslop` skill: take the lowest-numbered open ticket whose area does not start with `Needs owner ·`. After DSLP-5, work the open P1 and P2 findings, then fill in `## Report` below.

## Report

Ground base: `e081d90fb98020ba64b307da178ccf3068e7e73e` (ancestor of
`30c82c502db9b630eac1482fc7bcbe2349d8fb47`; confirmed with
`git merge-base --is-ancestor`). The cumulative review covers renderer and
scrl, including terminal restoration, signals, untrusted controls and pager
input. It found no additional demonstrated defect. Real PTY quit and SIGTERM
checks cover cleanup; live-loop interleavings, resize, full renderer formats
and unknown external consumers remain unverified.

| Measure | Before | After / qualification |
| --- | --- | --- |
| Rust source / test lines | 5,217 / 1,218 | 5,146 / 2,495; physical lines in `src`, `scrl/src`, `tests`, `scrl/tests`, with inline `mod tests` counted as tests; benchmarks excluded |
| Warm `make test` | 1.40s | 1.19s paired median at #31 versus 1.19s baseline; not a current-head paired measurement. Cold Ground: 15.41s; lint: 3.45s |
| Line / branch coverage | 3,025/4,182; 542/918 | 3,681/4,529; 648/930 at the recorded integrated revision, not HEAD; changed denominators limit comparison ([evidence](docs/deslop-cli-tests.md#current-workspace-branch-coverage)) |
| Mutation | Whole-workspace advisory: 497 caught, 439 missed, 48 unconfirmed timeouts, 53 unviable; 109 not run | Required #31 changed-source/protected-region gate qualified on Linux and macOS, 2,276 runner-seconds (37.9 runner-minutes) across 12 successful jobs; negative controls #36–39. No aggregate outcome count recorded. Advisory complete selections: CLI 22/0/0/2, numeric 37/0/1 confirmed hang/0, controls 70/8 equivalent/0/1, navigation 43/0/0/1 (caught/missed/timeout/unviable); not a whole-workspace pass |
| Escape-hatch lines | 139 | 133, Types measurement; six invariant `expect`s removed |
| Dead code | Buffered source pager branch and root `regex-lite` dependency | #47 removed 104 net session lines and 6 terminal lines, including the unreachable runner and exclusive adapter; removed one root dependency, retained scrl's use. Public retained-document runner remains |
| Findings | Diagnose filed three P3 findings (DSLP-6–8); three P2 step tickets | P3 DSLP-7 and DSLP-8 closed; DSLP-6 remains open; Bench filed P3 DSLP-10 and DSLP-11. Three P2 step tickets closed. No P1 findings recorded. Review filed/fixed none |
| Performance | Existing large-diff workload | Perf gate deferred: host busy. Types and sanitizer slices have separately qualified historical comparisons; #48 added tiered entrypoint and #49 added checked CLI rendering and PTY journeys. No current cumulative quiet-host verdict |
| Escaped regressions | Alignment boundary backtrace and uncolored bare controls lacked witnesses | Both pre-existing defects fixed with regression oracles (#33, #34); no newly escaped regression established by this review |

Per step (wall time / CI minutes are unknown unless stated):

- Ground #24: established base, test map and CI `make test`; 15.41s cold,
  1.40s warm, 3.45s lint locally.
- Diagnose #25: filed the three P3 test, type and liveness findings.
- Types #26: grouped filter and source into one state, removed six expects;
  rejected all strict-profile candidates for false positives or no witness.
  Five interleaved benchmark pairs showed no regression under the recorded
  rule ([evidence](docs/deslop-types.md)).
- Tests #31, #41–46: protected literal CLI output and errors, streaming,
  controls, numeric layout, pager frames, navigation, signal cleanup and input
  lifetime. Required gate cost is above; closeout spent zero new runner-minutes.
  The older 1,200-second advisory workspace run was incomplete. Coverage and
  mutation details and limitations are in [test evidence](docs/deslop-cli-tests.md).
- Dead code #47: traced source dispatch and consumers, removed the unreachable
  branch and root dependency; workspace tests, lint and dependency scan passed.
- Bench #48–49: added Edit/Change entrypoint and checked process journeys;
  Full was not run. Remaining benchmark oracles, attribution and comparison
  validity are DSLP-10 and DSLP-11.
- Findings and Report: alignment and bare-control fixes landed with witnesses;
  cumulative review found no new demonstrated issue. Costs for individual
  steps and other CI jobs were not recorded, so cannot be totaled.

Method feedback: exact public-boundary oracles exposed both the missing
alignment operations and bare-control escape; require those witnesses before
counting a boundary protected. The broad mutation run exhausted its budget;
use bounded, reconciled selections and distinguish the required gate from
advisory results. Strict-profile candidates found no adoptable setting.
Shared-file gaps: mutation recipes and report templates should preserve scope,
result classes and runner cost, and benchmark guidance should distinguish a
saved baseline from interleaved quiet-host comparisons. No shared files were
changed. Open tickets are P3 only.

## Tickets











### DSLP-6 · P3 · Guard CLI success and failure contracts at the process boundary

- **Area:** CLI · tests
- **Where:** tests/cli.rs:4; src/main.rs:20; scrl/src/main.rs:54; scrl/tests/auto_paging.rs:13
- **Problem:** Test-gap finding: the root CLI process test only rejects an invalid paging value, while valid paging flags, rendered output, successful exit, and I/O failure exit are not asserted through the executable. The scrl process test covers short auto paging but not its other flags or error exits. Regressions in argument wiring or status mapping can therefore pass the workspace suite.
- **Recommendation:** Add bounded process-level scenarios for valid direct output and representative invalid input and I/O errors for both commands, with literal output and exit assertions. Include src/main.rs and scrl/src/main.rs in the Tests step's critical mutation scope.
- **Fix with:** test-audit
- **Effort:** M


### DSLP-10 · P3 · Broaden CLI journey measurements

- **Area:** Bench · user journeys
- **Where:** benches/bench.rs
- **Problem:** Redirected rendering and a PTY paging/search journey now cover one fixture each. Other fixture sizes, child CPU and peak memory, and broader render correctness oracles remain unmeasured.
- **Recommendation:** Expand coverage when a concrete regression or workload warrants it. P3 because representative process boundaries now have checked measurements.
- **Fix with:** refactor-benchmarks
- **Effort:** M


### DSLP-11 · P3 · Qualify benchmark comparisons and attribution

- **Area:** Bench · measurement
- **Where:** benches/bench.rs; docs/development.md
- **Problem:** Steady-state allocator counts and elapsed samples do not explain CPU, peak footprint or I/O, and the persisted host baseline is not an interleaved gated reference. Fixture and host changes can confound comparisons.
- **Recommendation:** Qualify interleaved comparisons and fixture identity, and document separate CPU sampling, allocation and syscall attribution runs with their overhead.
- **Fix with:** benchmark-checklist
- **Effort:** M
