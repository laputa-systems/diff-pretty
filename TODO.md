Prefix: DIFF
Next: 9
### DIFF-4 · P3 · Guard CLI success and failure contracts at the process boundary

- **Area:** CLI · tests
- **Where:** tests/cli.rs:4; src/main.rs:20; scrl/src/main.rs:54; scrl/tests/auto_paging.rs:13
- **Problem:** Both CLI suites now assert valid output, paging options, help and representative read and write errors, and the mutation gate includes both entry points. Remaining process-level edge cases and untested terminal interleavings can still regress without a focused witness.
- **Recommendation:** Audit the CLI process boundaries for user-visible gaps and add bounded literal-output or exit-status scenarios only where existing tests do not protect the contract.
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

### DIFF-7 · P3 · Cover remaining renderer and pager behavior boundaries

- **Area:** Renderer and pager · tests
- **Where:** src/render.rs; scrl/src/session.rs; scrl/src/terminal.rs; scrl/src/document.rs; scrl/src/search.rs; scrl/src/source.rs; tests/pager_entry.rs
- **Problem:** Existing literal renderer, session, source and PTY tests do not establish full diff-format or exported API coverage, every search and streaming error decision, live-loop interleaving, resize or terminal lifecycle behavior. Wider mutation and coverage inventories are incomplete, so regressions in those paths may go undetected.
- **Recommendation:** Identify reachable user-visible gaps in these paths, add focused behavioral witnesses or document dispositions, and refresh coverage and targeted mutation results without treating incomplete runs as passes.
- **Fix with:** test-audit
- **Effort:** L

### DIFF-8 · P3 · Qualify current test and renderer performance

- **Area:** Performance · verification
- **Where:** Makefile; benches/bench.rs; docs/development.md
- **Problem:** Quiet-host paired test-cost and cumulative renderer runtime measurements at the integrated revision were deferred. A historical corpus regression at dca96f1 against 49a1c97 had no established cause, while a later candidate comparison did not reproduce it. Linux performance was also unmeasured.
- **Recommendation:** When a comparison is needed, measure representative test and renderer workloads against an appropriate baseline with interleaved runs and host gates. Investigate the historical regression if it reproduces, and assess Linux separately if it is in scope.
- **Fix with:** benchmark-checklist
- **Effort:** M
