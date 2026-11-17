# Session draw boundary

Scope: public `Session::draw`, including help routed through it; no terminal loop, resize, renderer or classifier inventory. Base revision `5285283d20f1cecdf6a89b688d351a9e1140d829`; base session SHA-256 `f4550d691a16075f54d2ab126bc82cc21040c9a949461f08f025cf6c2884e8ad`. Tests use literal whole frames, including status and help at one and twelve rows, search input, repeat drawing, and real `Write` failures for both modes. An unchanged event returns `changed: false`; an explicit repeated draw emits the same complete frame. The caller decides whether to draw after an event.

Environment: macOS arm64, rustc 1.102.0-nightly (5b7b37aef 2026-11-08), cargo 1.102.0-nightly (c07ac2c4d 2026-11-06), cargo-mutants 27.1.0. Cargo.lock SHA-256 `11fa85bf38485289c3efc7e6e406251179f8a4ba0e71130820e1d33ffb72d2c8`. Commands below run from repository root with `PATH=/Users/josh/d/stuff/.work/tools/bin:$PATH`, `TMPDIR=$PWD/.work/tmp`, `RUSTC_WRAPPER=kache`, `CARGO_BUILD_JOBS=1`; create `.work/tmp` first. Ordinary and mutation targets are distinct. No benchmark was run.

## Reproduction and results

The candidate `scrl/src/session.rs` SHA-256 after restoration is `98de3a8a5b39c7ef06b3cb4a9202dd5e98108e2f60919f82bcee6eea42c0b394`. The focused command below passed three scenarios (exit 0). An initial test-authoring run exited 101 because the expected 40-column status omitted `PgU`; the literal was corrected before the mutation baseline.

```sh
export PATH=/Users/josh/d/stuff/.work/tools/bin:$PATH TMPDIR=$PWD/.work/tmp RUSTC_WRAPPER=kache CARGO_BUILD_JOBS=1
mkdir -p "$TMPDIR" .work/draw-mutation
RUST_TEST_THREADS=1 CARGO_TARGET_DIR=$PWD/.work/target-draw-final cpu-guard --cores 2 -- cargo test -p scrl public_draw -- --nocapture
```

Hand fault: replace the normal draw's `output.flush()` with `let _ = output.flush(); Ok(())`, then run the following command. It exited 101: `public_draw_propagates_write_and_flush_errors_in_both_modes` called `unwrap_err` on `Ok(())`. Restore the original line and verify the candidate hash above. The fault was not retained.

```sh
RUST_TEST_THREADS=1 CARGO_TARGET_DIR=$PWD/.work/target-draw-final cpu-guard --cores 2 -- cargo test -p scrl public_draw_propagates_write_and_flush_errors_in_both_modes
```

Complete generated mutation selection, including return and expression mutants, used the fresh baseline and no exclusions. The lock is shared with other repositories; wait at most 300 seconds, release it on shell exit. The outer command exited 2, cargo-mutants' missed-mutant result. Baseline passed (42 scrl unit tests plus integration tests); 14 generated: 13 caught, one missed, zero unviable, zero timeout. The complete output has 15 logs including baseline. All identities and logs were inspected; the caught cases include arithmetic underflow panics as well as semantic frame assertions, and are not all independent semantic detections.

```sh
RUST_TEST_THREADS=1 CARGO_TARGET_DIR=$PWD/.work/target-draw-mutation timeout -k 5 1200 sh -c 'n=0; until mkdir /Users/josh/d/stuff/.work/tmp/mutation.lock 2>/dev/null; do n=$((n+1)); [ "$n" -lt 300 ] || exit 75; sleep 1; done; trap "rmdir /Users/josh/d/stuff/.work/tmp/mutation.lock" EXIT; cpu-guard --cores 2 -- cargo mutants --in-place --baseline run --timeout 60 --build-timeout 300 --no-shuffle --skip-calls-defaults false -p scrl -f scrl/src/session.rs -F "Session::(draw|draw_help)" -o .work/draw-mutation'
```

Caught identities, using original source line/column: draw return at 377:9; search row `+` to `-` and `*` at 390:77; command capacity `+` to `-` at 398:70; help return at 426:9; help newline comparison `<` to `==`, `>`, `<=` at 445:24 and row `+` to `-`, `*` at 445:20; help status `>` to `==`, `<`, `>=` at 449:27. The sole survivor, `398:70` `+` to `*`, changes `String::with_capacity(input.text.len() + 1)` to capacity `len * 1`. Both construct the same command via subsequent pushes; capacity changes allocation only. For representable allocated strings neither arithmetic expression changes output or error behavior. It is functionally equivalent, not an excluded mutant.

Final verification, all exit 0, using restored source and separate ordinary target:

```sh
RUST_TEST_THREADS=2 CARGO_TARGET_DIR=$PWD/.work/target-draw-final cpu-guard --cores 2 -- make test
RUST_TEST_THREADS=2 CARGO_TARGET_DIR=$PWD/.work/target-draw-final cpu-guard --cores 2 -- make lint
RUST_TEST_THREADS=2 CARGO_TARGET_DIR=$PWD/.work/target-draw-final cpu-guard --cores 2 -- cargo clippy --workspace --all-targets --all-features -- --deny warnings
```

The release workspace run passed 87 executable tests and one doctest, zero ignored. No missing-tool skip was reported. This is a Darwin run; Linux and other platform-gated paths were not run here. It does not establish live-loop, draw performance, full API, renderer format, or whole-workspace mutation coverage. Broader DSLP4 remains open.
