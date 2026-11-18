# Sanitizer cost at 3653bb3

The current candidate meets the recorded runtime rule against `49a1c97` on the original benchmark selection. No renderer change was made in this check. The earlier `dca96f1` corpus regression remains a historical observation, not a measurement of `3653bb3`. The new comparison does not establish why the earlier regression occurred. The separate symbolized profile in [the controls receipt](deslop-controls-performance.json) attributes most sampled work to alignment and cannot isolate the control scan. A further three-second candidate profile likewise found 29 samples at the scan call site and alignment dominated. Neither profile establishes a causal cost.

## Runtime

On macOS arm64, the dist builds used the pinned toolchain, fat LTO, panic abort, stripped symbols and the flags recorded in [the original receipt](deslop-tests-performance.json). The benchmark source, fixtures and toolchain files have no diff between `49a1c97` and `3653bb3`. The retained baseline executable was read without rebuilding its tree. Each package had five pairs, rotating side order, one warmup and one measured run per side, five samples of ten iterations. `perf-gate-ready` and immediate `perf-gate-ready -1` both exited 0 for every one of the 20 measurements. The benchmark wrapper exited 0 for each. The rule fails a row when its positive median delta exceeds both 2% of baseline and the larger within-side range. No row failed.

The corpus median was 5,064,783 ns baseline and 5,071,762 ns candidate. Baseline run medians were 4,866,871, 5,156,303, 4,925,458, 5,064,783 and 5,132,600 ns. Candidate run medians were 5,083,804, 5,071,762, 5,007,963, 5,010,242 and 5,621,644 ns. The 6,979 ns difference is below the 613,681 ns larger range. Both sides allocated 6,970 times and 15,174,210 bytes per corpus iteration. This is no measurable difference under the rule, not evidence of a speedup. The profile identifies alignment as the dominant sampled CPU work, but does not establish the cause of the historical regression.

The table lists median nanoseconds and the larger side's range for every original row.

| Benchmark | Baseline | Candidate | Range |
| --- | ---: | ---: | ---: |
| bench::pager_viewport_show_010 | 3600 | 3566 | 380 |
| bench::render_document_show_010 | 64823 | 64110 | 7518 |
| bench::render_log_000 | 358792 | 360297 | 19537 |
| bench::render_log_000_color | 399720 | 401328 | 22853 |
| bench::render_plain_unified | 1393 | 1378 | 279 |
| bench::render_reader_document_synthetic_git_log_10000 | 10302443 | 9707306 | 558221 |
| bench::render_reader_to_log_000 | 363083 | 369637 | 12966 |
| bench::render_show_corpus | 5064783 | 5071762 | 613681 |
| bench::render_show_large | 1560821 | 1571023 | 91809 |
| bench::render_show_metadata | 329779 | 347094 | 46873 |
| bench::render_show_typical | 2911661 | 2963680 | 174443 |
| bench::cached_search_redraw | 10133 | 9283 | 2108 |
| bench::filtered_stream_and_redraw_500k | 1770641 | 1773996 | 300723 |
| bench::filtered_stream_and_redraw_50k | 171250 | 169762 | 26488 |
| bench::filtered_stream_and_redraw_5m | 18116212 | 18393975 | 2921109 |
| bench::initial_regex_search_late_match | 2496062 | 2531466 | 718179 |
| bench::initial_search_late_match | 2392109 | 2472479 | 708379 |
| bench::live_search_chunk_and_redraw | 1220271 | 1217212 | 224979 |
| bench::session_redraw_without_movement | 12746 | 12412 | 3509 |
| bench::viewport_redraw_horizontal_clip | 10921 | 10900 | 1370 |
| bench::viewport_redraw_no_search | 12425 | 12237 | 2471 |
| bench::viewport_redraw_styled | 9050 | 8971 | 962 |

The first direct-executable attempt exited 2 because `rustybench` passed Cargo's `--` separator to the executable. Its gates exited 0, but its results were discarded. The adapter below removes that separator.

To reproduce, build each side with the configuration from the original receipt. Use the retained baseline build or build the baseline in isolated scratch. Set `base` and `candidate` to the two benchmark executables for each package. The baseline executable must retain its matching fixture tree. From the candidate tree, run the following in zsh once for `diff-pretty` and once for `scrl`:

```sh
export RUSTC_WRAPPER=kache CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 TMPDIR="$PWD/.work/tmp"
cat > .work/tmp/bench-wrapper <<'SH'
#!/bin/zsh
exe=$1
shift 2
exec "$exe" "$@"
SH
for i in 1 2 3 4 5; do
  if (( i % 2 )); then sides=(base candidate); else sides=(candidate base); fi
  for side in $sides; do
    if [[ $side == base ]]; then exe=$base; else exe=$candidate; fi
    perf-gate-ready > .work/tmp/$package-$i-$side.pre 2>&1 || exit 50
    cpu-guard --cores 1 -- rustybench baseline \
      --baseline "$PWD/.work/tmp/$package-$i-$side.json" \
      --warmup 1 --runs 1 --sample-count 5 --sample-size 10 \
      -- zsh "$PWD/.work/tmp/bench-wrapper" "$exe" > .work/tmp/$package-$i-$side.log 2>&1
    run=$?
    perf-gate-ready -1 > .work/tmp/$package-$i-$side.post 2>&1
    post=$?
    (( run == 0 && post == 0 )) || exit 51
  done
done
```

Use `jaq -s` on the five baseline JSON files followed by the five candidate JSON files. Group `.benchmarks[]` by name, take the median of each side's five `median_ns` values, and compare the delta with 2% of the baseline and the larger range. Reject a pair if either gate fails. The renderer benchmark uses the result of each render, as shown in [the benchmark](../benches/bench.rs).

## Correctness and resources

The existing CLI oracle in [tests/cli.rs](../tests/cli.rs) passed on the candidate. A hand fault replacing the control predicate at `render_chunk` with the pre-fix ESC-only check made that same test fail at its uncolored NUL and C1 assertion. Restoring the source made it pass. The fault test exited 101 and the restored test exited 0. The shared mutation lock was held during the fault and released afterward. A separate uncolored hunk containing bare C0 and C1 controls, excluding tab, line delimiters and ESC, differed from its clean version on the baseline and matched on the candidate. Both binaries exited 0. All 103 fixture outputs matched byte for byte.

`make test` and `make lint` exited 0 with one Cargo job, one test thread, `kache` and `cpu-guard --cores 1`. Binary sizes were 285,504 bytes for each `diff-pretty` and 252,416 bytes for each `scrl`. Five redirected corpus observations per side had maximum RSS ranges of 4,702,208–4,849,664 bytes baseline and 4,669,440–4,734,976 candidate for `diff-pretty`, and 2,064,384 baseline and 2,048,000–2,064,384 candidate for `scrl`. Every paired output matched. Five early-exit PTY observations per side used the original receipt's script. All exited 0, sent `q`, and produced matching frame hashes. Their RSS ranges were 2,572,288–2,605,056 bytes for both `diff-pretty` sides, and 2,293,760–2,310,144 baseline versus 2,293,760 candidate for `scrl`. These RSS observations do not establish a memory reduction or measure a full pager traversal. Linux was not measured.

Build the candidate benchmark executables with:

```sh
env CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 RUSTC_WRAPPER=kache \
  TMPDIR="$PWD/.work/tmp" CARGO_TARGET_DIR="$PWD/.work/tmp/perf-target" \
  CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=clang \
  RUSTFLAGS='-Zlocation-detail=none -Zunstable-options -Cpanic=immediate-abort -Clink-arg=-Wl,-dead_strip_dylibs' \
  cpu-guard --cores 1 -- cargo bench --workspace --bench bench --profile dist \
  -Z build-std=std -Z build-std-features= -Z panic-abort-tests \
  --target aarch64-apple-darwin --no-run
```

Run the fixture comparison with each side's dist `diff-pretty` path as `base` and `candidate`:

```sh
for f in fixtures/*.patch; do
  "$base" --no-pager < "$f" > .work/tmp/base.out || exit 1
  "$candidate" --no-pager < "$f" > .work/tmp/candidate.out || exit 1
  cmp .work/tmp/base.out .work/tmp/candidate.out || exit 1
done
```

For the fault witness, acquire `/Users/josh/d/stuff/.work/tmp/mutation.lock` with `mkdir`, save `src/render.rs` in scratch, and restore it before releasing the lock. Replace the single `raw_lines.iter().any(|line| has_terminal_controls(line))` condition with `input.as_bytes().contains(&b'\x1b')`. Run the command below before and after restoration. Use a separate target for the fault and run no other build in the worktree while the fault is present.

```sh
env CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 RUSTC_WRAPPER=kache \
  TMPDIR="$PWD/.work/tmp" CARGO_TARGET_DIR="$PWD/.work/tmp/fault-target" \
  cpu-guard --cores 1 -- cargo test -p diff-pretty --test cli \
  terminal_controls_preserve_literal_metadata_and_hunk_text
```

For the resource observations, concatenate `fixtures/*.patch` into scratch, compare the two dist executables' redirected `--no-pager` outputs, and record `/usr/bin/time -l` maximum RSS. Extract `.interactive_rss.pty_script` from the original receipt with `jaq -r` and run it in a fresh Python process for each executable and side. The original receipt describes the terminal and input. No elapsed time from these resource observations is a timing claim.
