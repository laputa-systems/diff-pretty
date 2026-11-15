Prefix: DSLP
Next: 9

Deslop campaign backlog ([plan](https://github.com/laputa-systems/stuff/blob/main/docs/campaigns/deslop.md), tier A). One agent works this file with the `deslop` skill: take the lowest-numbered open ticket whose area does not start with `Needs owner ·`. After DSLP-5, work the open P1 and P2 findings, then fill in `## Report` below.

## Report

Ground base: `e081d90fb98020ba64b307da178ccf3068e7e73e`. Rust code: 6.0k lines (fleet/loc/v1, vendored included, seeded). On macOS with a fresh scratch target, guarded `make test` took 15.41s cold and 1.40s warm; guarded `make lint` took 3.45s. All passed. PR CI now calls `make test`; platform-specific terminal tests run on their matching OS, and no missing-tool skip was found. No vendored trees found. The existing large-diff rendering benchmark is the declared perf workload; later scorecard measurements are pending.

## Tickets



### DSLP-3 · P2 · Types: strict profile and typing findings

- **Area:** deslop · types
- **Where:** type checker and lint configuration; Rust sources
- **Problem:** The strict-profile candidates in the deslop skill's tools reference have not been measured here.
- **Recommendation:** Run the deslop Types step. Accept: clean at the adopted profile, escape hatches not increased, witnesses recorded.
- **Fix with:** deslop
- **Effort:** M

### DSLP-4 · P2 · Tests: coverage, boundary gaps, mutation

- **Area:** deslop · tests
- **Where:** test suites and CI
- **Problem:** Coverage and mutation results are unmeasured; no deslop mutation lane exists.
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


### DSLP-7 · P3 · Represent filtered session state together

- **Area:** pager · types
- **Where:** scrl/src/session.rs:73; scrl/src/session.rs:158; scrl/src/session.rs:216
- **Problem:** Typing finding: filter and source_document are independent Options although filtered sessions require both. Session::push_chunk relies on repeated expect calls for this private invariant; constructors currently establish it, so a reachable failure is unproven. A future internal change can construct a filtered session without its source document and compile successfully.
- **Recommendation:** Evaluate a single filtered-state variant containing both the regex and source document, preserving the public API and behavior. Use the currently accepted mismatched internal construction as a negative type witness and remove the corresponding expects if the model remains simpler.
- **Fix with:** typing-audit
- **Effort:** S


### DSLP-8 · P3 · Investigate unreachable legacy terminal runner branch

- **Area:** pager · dead code
- **Where:** scrl/src/lib.rs:60; scrl/src/session.rs:595; scrl/src/session.rs:608
- **Problem:** Dead-code candidate: run_source returns before calling run_terminal when paging is Never, and Always or Auto immediately dispatch to run_terminal_live. The subsequent retained-source branch appears unreachable through the sole production caller. Its liveness across tests, features, and platforms is unproven, and it duplicates terminal lifecycle logic.
- **Recommendation:** Trace all callers and supported feature and platform configurations; if no caller reaches the branch, remove it and its exclusively used helpers while preserving the public runner and testing its consumers.
- **Fix with:** dead-code-archaeology
- **Effort:** S

