Prefix: DIFF
Next: 7
### DIFF-4 · P3 · Guard CLI success and failure contracts at the process boundary

- **Area:** CLI · tests
- **Where:** tests/cli.rs:4; src/main.rs:20; scrl/src/main.rs:54; scrl/tests/auto_paging.rs:13
- **Problem:** Test-gap finding: the root CLI process test only rejects an invalid paging value, while valid paging flags, rendered output, successful exit, and I/O failure exit are not asserted through the executable. The scrl process test covers short auto paging but not its other flags or error exits. Regressions in argument wiring or status mapping can therefore pass the workspace suite.
- **Recommendation:** Add bounded process-level scenarios for valid direct output and representative invalid input and I/O errors for both commands, with literal output and exit assertions. Include src/main.rs and scrl/src/main.rs in the Tests step's critical mutation scope.
- **Fix with:** test-audit
- **Effort:** M

### DIFF-5 · P3 · Broaden CLI journey measurements

- **Area:** Bench · user journeys
- **Where:** benches/bench.rs
- **Problem:** Redirected rendering and a PTY paging/search journey now cover one fixture each. Other fixture sizes, child CPU and peak memory, and broader render correctness oracles remain unmeasured.
- **Recommendation:** Expand coverage when a concrete regression or workload warrants it. P3 because representative process boundaries now have checked measurements.
- **Fix with:** refactor-benchmarks
- **Effort:** M

### DIFF-6 · P3 · Qualify benchmark comparisons and attribution

- **Area:** Bench · measurement
- **Where:** benches/bench.rs; docs/development.md
- **Problem:** Steady-state allocator counts and elapsed samples do not explain CPU, peak footprint or I/O, and the persisted host baseline is not an interleaved gated reference. Fixture and host changes can confound comparisons.
- **Recommendation:** Qualify interleaved comparisons and fixture identity, and document separate CPU sampling, allocation and syscall attribution runs with their overhead.
- **Fix with:** benchmark-checklist
- **Effort:** M

