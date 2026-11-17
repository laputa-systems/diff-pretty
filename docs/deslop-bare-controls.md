# Bare controls in uncolored hunks

The renderer strips Unicode terminal controls other than tab before parsing hunks, then expands tabs in hunk text. Leading metadata retains ordinary text and tabs while terminal controls are removed. Previously an ESC-only fast path skipped stripping for uncolored hunks, leaking bare C0, DEL and C1 characters. The CLI regression in `tests/cli.rs` exercises that path and colored metadata and hunk input with literal output; its subprocess has a ten-second deadline and is killed and reaped on failure.

This slice starts at `aa0e5053aced5644db254acdc0e392a517abb7d2`; its production renderer blob is `9298907e620d6292363d6c81ae308e6dae2f4432`, matching the production portion of `f17f82f72c9fc55a2abd292c8ef96e9ab151b54d`. The baseline regression exited 101 with NUL, DEL and C1 bytes in output; after the patch it exited 0. `make test` and `make lint` exited 0. These commands ran with `TMPDIR="$PWD/.work/tmp" CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 RUSTC_WRAPPER=kache` and `cpu-guard --cores 1 --`:

```sh
cargo test -p diff-pretty --test cli terminal_controls_preserve_metadata_and_hunk_text
make test
make lint
```

## Cost and compatibility

On macOS arm64, the retained dist baseline is `49a1c97d1f60932c3d860318672ecdcc545a0986`. Benchmark sources, fixtures, toolchain and Cargo configuration match this slice; its manifests and lockfile also match the baseline. The broader candidate `3653bb34c802447631e46e16abfb198f849fd8ce` adds a dev dependency and has a CLI helper extraction, so its [earlier measurements](https://github.com/laputa-systems/diff-pretty/blob/6f1719d8f0bb1ff9ab42ea0fcc91994ea9909ccb/docs/deslop-sanitizer-cost.md) are not used as this candidate's timing proof. The older 7.97% observation remains historical; its cause is unverified. Benchmark executables from that candidate differ in hash from the builds below. The candidate feature graph was recorded with `cargo tree --workspace -e features --locked` (SHA-256 `0f263a7d3abb271027edeaed3527b51a06fd5fb979a2927f4be49d4cf73f205f`).

The baseline and candidate `diff-pretty` benchmark executable SHA-256 values were `dc301617088ab9c67f549a52dce69dacabe40e04cbfb846ed29ea8b2f1757852` and `807a0efb14c99955f16eb8113f5ae5508841e59a5f6b78e1b1a3a4cbe904921d`; the corresponding `scrl` benchmark hashes were `e58a9ef4fece1da1f9faad69ab4b6a26f01374a8f53227d6e8fbe68f26c89126` and `10a698aca51bca13d7c9265ae6384001ca9cf9d95096c7a7030832c20515c2ac`. Five alternating pairs per package, one warmup and one run of five samples of ten iterations, all passed `perf-gate-ready` before and `perf-gate-ready -1` immediately after each run. All 20 benchmark commands exited 0. A row fails only if its median increase exceeds both 2% of baseline and the larger range of the five run medians. All 22 rows passed; allocation counts and bytes were unchanged across all runs. Values below are nanoseconds: baseline median / candidate median / larger range.

| Benchmark | Baseline | Candidate | Range |
| --- | ---: | ---: | ---: |
| pager_viewport_show_010 | 3654 | 3975 | 1242 |
| render_document_show_010 | 63393 | 68208 | 11422 |
| render_log_000 | 374190 | 395879 | 100534 |
| render_log_000_color | 456119 | 446349 | 115167 |
| render_plain_unified | 1479 | 1687 | 771 |
| render_reader_document_synthetic_git_log_10000 | 11053784 | 10677297 | 1380448 |
| render_reader_to_log_000 | 401389 | 399396 | 111573 |
| render_show_corpus | 5407228 | 5826517 | 756198 |
| render_show_large | 1714285 | 1814100 | 314631 |
| render_show_metadata | 365731 | 396387 | 98533 |
| render_show_typical | 3130212 | 3537752 | 639658 |
| cached_search_redraw | 11346 | 10016 | 3267 |
| filtered_stream_and_redraw_500k | 2084747 | 2042658 | 399403 |
| filtered_stream_and_redraw_50k | 211522 | 206212 | 67583 |
| filtered_stream_and_redraw_5m | 22095072 | 22640021 | 3534678 |
| initial_regex_search_late_match | 3179741 | 2891641 | 694125 |
| initial_search_late_match | 2974691 | 2800412 | 833820 |
| live_search_chunk_and_redraw | 1622325 | 1342837 | 424862 |
| session_redraw_without_movement | 16925 | 15437 | 4896 |
| viewport_redraw_horizontal_clip | 14150 | 13583 | 2758 |
| viewport_redraw_no_search | 14466 | 14775 | 3420 |
| viewport_redraw_styled | 10704 | 10646 | 1859 |

The actual baseline and candidate `diff-pretty` SHA-256 hashes were `9c30e2d42964bf64687749201ef7d8e3f6ac7ae12c9015e818c32a4c065a7381` and `b0f673ffb1ab2fd9ff07e1b4b104db8de59735b6f63c053dd3c9d70404b88d4a`; both were 285504 bytes. Both `scrl` binaries were 252416 bytes and had SHA-256 `c9c0242c464ac524420fac6b56c6f63d481a58e80708c47a3084d17d2d80fbb3`. All 103 fixture outputs matched. Five redirected corpus runs per side matched output: `diff-pretty` RSS ranges were 4718592–4751360 baseline and 4636672–4816896 candidate bytes; `scrl` was 2048000 on both sides. Five early-exit PTY runs per side all exited 0 after `q`, with matching frame hashes. RSS ranges were 2588672–2621440 baseline and 2539520–2572288 candidate for `diff-pretty`, and 2293760 baseline versus 2293760–2326528 candidate for `scrl`. These are observations, not full pager traversal or Linux measurements.

## Reproduction

The retained baseline executable and benchmark fixture tree are under `.work/wt/ds-perf-base/.work/perf-abort-target` in the repository's parent worktree area. Build the candidate in this worktree, with the same dist profile and flags:

```sh
mkdir -p .work/tmp
export TMPDIR="$PWD/.work/tmp" CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 RUSTC_WRAPPER=kache
export CARGO_TARGET_DIR="$PWD/.work/tmp/perf-target" CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=clang
export RUSTFLAGS='-Zlocation-detail=none -Zunstable-options -Cpanic=immediate-abort -Clink-arg=-Wl,-dead_strip_dylibs'
cpu-guard --cores 1 -- cargo bench --workspace --bench bench --profile dist -Z build-std=std -Z build-std-features= -Z panic-abort-tests --target aarch64-apple-darwin --no-run
```

Use the baseline and candidate benchmark executables in their respective `dist/build/<package>/*/out/bench-*` directories. For each package run the following zsh loop, setting `base`, `candidate`, and `package` to those executable paths and package name. The wrapper removes Cargo's `--` separator. Reject a measurement if either quiet check fails.

```sh
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
    cpu-guard --cores 1 -- rustybench baseline --baseline "$PWD/.work/tmp/$package-$i-$side.json" --warmup 1 --runs 1 --sample-count 5 --sample-size 10 -- zsh "$PWD/.work/tmp/bench-wrapper" "$exe" > .work/tmp/$package-$i-$side.log 2>&1
    run=$?
    perf-gate-ready -1 > .work/tmp/$package-$i-$side.post 2>&1
    post=$?
    (( run == 0 && post == 0 )) || exit 51
  done
done
```

Concatenate the five baseline JSON files followed by the five candidate files and use `jaq -s` on standard input; group `.benchmarks[]` by name, compare each side's median `median_ns` and larger range as above, and compare `alloc_count` and `alloc_bytes`. Compare each fixture with both `diff-pretty --no-pager` executables using `cmp`. For redirected RSS, concatenate `fixtures/*.patch`, run each binary five times under `/usr/bin/time -l` with `diff-pretty --no-pager` reading the corpus or `scrl --paging=never <corpus>`, and compare output. For PTY RSS, extract `.interactive_rss.pty_script` with `jaq -r` from the [original receipt](https://github.com/laputa-systems/diff-pretty/blob/6f1719d8f0bb1ff9ab42ea0fcc91994ea9909ccb/docs/deslop-tests-performance.json) to a file whose name is not `pty.py`, then invoke it in a fresh Python process five times per executable with the corpus path. It enforces a deadline, sends `q`, records the frame hash and RSS, and kills and reaps on timeout.
