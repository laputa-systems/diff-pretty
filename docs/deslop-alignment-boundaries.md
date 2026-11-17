# Alignment boundary evidence

Status: incomplete. DSLP-9 remains open. The required workspace Clippy check fails in unchanged `scrl/src/lib.rs:165` on both baseline and candidate. The assigned scope does not permit editing that file. Performance and memory observations are recorded below. Complete acceptance remains unverified.

## Change and consumers

Admission revision: `35eae8f07c9c571e153a6bd026751fea5ebf1dd7`. Runtime fix: `f1f2a95`. Test commits: `8996cbb` and `250b572`. Comparison baseline: `6ba0962f002cd0f03ad50ede43216bf581a8c228`. Candidate `src/align.rs` SHA-256: `fcf5732838826cf7fc13740f4977a5937591a92b6437d721aba57ce539402695`. Candidate test SHA-256: `40f4190680f080dc101da55973fcfd31dc8afd2c9dc548c9706ffdffcc3806a8`.

Boundary cells now point to the preceding cell, and an empty backtrace stops before the origin. Empty public inputs return no runs. One-sided inputs return a run containing every token. Costs and candidate ordering are unchanged. `src/edits.rs:21-23` prepends an empty sentinel for first-party tokenization. `src/edits.rs:129` consumes the runs, and `src/edits.rs:295` and `:416` construct alignments for edit inference. External consumers are unknown. Cargo metadata identified both workspace manifests under this candidate checkout.

## Correctness checks

All Cargo commands used `PATH=/Users/josh/d/stuff/.work/tools/bin:$PATH`, `TMPDIR=$PWD/.work/tmp`, `RUSTC_WRAPPER=kache`, `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, and `cpu-guard --cores 2`. Ordinary checks used `.work/target-alignment-final` as `CARGO_TARGET_DIR`.

- Before the fix, `cargo test --test alignment_api` exited 101: empty inputs produced `[(NoOp, 1)]` and three left tokens produced `[(Deletion, 1)]`.
- After the fix, the same command exited 0. Restoring both original boundary parents made it exit 101 at the one-sided literal; the source was restored afterward.
- The public child scenario checks 225 pairs over `{a,b}` with lengths zero through three. It enumerates complete paths independently, checks token consumption and equal matches, compares minimum cost, pins selected ties, and exercises reset and output-buffer reuse. The parent enforces a 15-second deadline and kills and reaps the child.
- `make test` exited 0 and `make lint` exited 0. `cargo clippy --workspace --all-targets --all-features -- --deny warnings` exited 101 on `scrl/src/lib.rs:165`, where `clippy::err_expect` rejects `.err().expect()`. The same command at the baseline revision exited 101 at the same line. Linux was not tested.

## Mutation

`cargo-mutants 27.1.0` ran under the shared mutation lock with a separate target and the outer `timeout -k 5 1200`. The full-package command was `cargo mutants --no-config --skip-calls-defaults=false --in-place --file src/align.rs --baseline run --timeout 60 --build-timeout 300 --output .work/mutation-results`. It selected 66 mutants, passed the fresh baseline, and exited 3: 56 caught, four missed, two unviable, and four timeouts. The timeouts occurred before the bounded integration scenario completed.

After adding two literal tie cases, the complete 66-mutant selection ran again with the same options and `--output .work/mutation-results-final -- --test alignment_api`. It passed the fresh baseline and exited 2: 62 caught, two missed, two unviable, zero timeouts. All caught logs contain a failure or panic. The earlier four timed-out mutations were caught in this bounded run. The two formerly missed cost mutations are caught by the new tie cases. The two unviable replacements require `Default` for `Alignment` and `Cell`, which those types do not implement.

The remaining survivors are functionally equivalent for returned operations, not performance-equivalent. `take_cells` returning an empty vector loses allocation reuse: `src/edits.rs:382` stores the result and `:295` passes it to a new alignment, whose `reset_buffers` grows the table and fills it before use. Changing `<` to `<=` at `src/align.rs:89` reallocates when the existing length equals the required size; both branches retain the required length and `fill` overwrites all non-origin cells. These dispositions do not establish equal allocation or timing behavior.

The generated inventory covers `new`, `x_tokens`, `y_tokens`, `with_cells`, `take_cells`, `reset_lines`, `reset_buffers`, `fill`, `mismatch_cost`, `coalesced_operations_into`, and `index`. The raw mutation logs are in ignored `.work/mutation-results` and `.work/mutation-results-final` for this run.

## Shipping build and corpus

Both sides built `diff-pretty` and `scrl` on Darwin arm64 with rustc `1.102.0-nightly (5b7b37aef 2026-11-08)`, clang, dist profile, `RUSTFLAGS='-Zlocation-detail=none -Zunstable-options -Cpanic=immediate-abort -Clink-arg=-Wl,-dead_strip_dylibs'`, and `cargo build -p diff-pretty -p scrl --profile dist -Z build-std=std -Z build-std-features= --target aarch64-apple-darwin`. Both package benchmark targets also built with `cargo bench -p PACKAGE --bench bench --profile dist -Z build-std=std -Z build-std-features= -Z panic-abort-tests --target aarch64-apple-darwin --no-run`. The builds exited 0. Both `diff-pretty` binaries are 285504 bytes and both `scrl` binaries are 252416 bytes. Size alone does not establish a performance result.

Each of 103 `fixtures/*.patch` inputs was passed to each `diff-pretty --no-pager`; all exits were zero, stderr was empty, and output bytes matched. Their individual output lengths sum to 681067 bytes. The concatenated input is 265220 bytes, SHA-256 `ee880641b3a23f02678a17fc6e95499f18c29598441adcea838d5bad5a1d8277`. Both rendered it to 680988 bytes, SHA-256 `2f0d7e67e7442742fc8a903a193d6d5e56147b7bc58feb4c951d9ff44db4f772`.

The workspace Clippy failure requires an out-of-scope correction before acceptance can be established.


## Matched performance observations

The baseline and candidate benchmark sources, fixtures, toolchain file, and lockfile have no differences. Benchmark source SHA-256 values are `87a10fd98c4e46cdeb8576992592af0a781fe9259f2c09663528cfa310d1fa43` for `benches/bench.rs` and `ef6a1dc3dec6b14be8162f1e2042027bae730ee1ac60eb4fed593ec9dd3bd5f7` for `scrl/benches/bench.rs`. `Cargo.lock` SHA-256 is `205e1dd6d5b4f07f7258bb9cc9fb02ede2c109c1205778dd8a6cc5f3c7fe3e61`. The installed rustybench CLI is 0.4.0 and the locked library is 0.5.0. The timed regions and output use are in `benches/bench.rs:66-105` and `scrl/benches/bench.rs:194-237`. Fixtures are loaded outside the timed renderer closures.

The pair script below ran from this checkout and exited 0. Each package ran separately with one warmup, one measured run, 15 samples, and sample size 20. The first, third, and fifth pairs ran baseline first; the second and fourth ran candidate first. All 20 pre-gates, 20 benchmark processes, and 20 immediate post-gates exited 0. The 20 logs contain no error or failure line. The host had 18 logical cores. The initial load averages were 4.39, 3.31, and 2.68. These actual gate exits, rather than process diagnostics, determine timing eligibility.

Each row below gives five per-run medians in nanoseconds in iteration order. `alloc` gives the per-operation allocation count and bytes reported on both sides. A regression requires the candidate median to exceed the baseline median by both 2% and the larger within-side range. None of the 22 rows met that condition. No speedup is claimed. Results reflect a single macOS host and this workload; they do not establish a CPU bottleneck or a general latency improvement.

| Package and benchmark | Baseline runs, ns | Candidate runs, ns | Allocations, count / bytes |
| --- | --- | --- | --- |
| diff-pretty `bench::pager_viewport_show_010` | 3777,3933,3498,3658,3650 | 3491,3646,3500,3644,3796 | 1 / 120 |
| diff-pretty `bench::render_document_show_010` | 72526,71972,71579,64358,66802 | 62869,67269,64368,63800,74433 | 65 / 62104 |
| diff-pretty `bench::render_log_000` | 384933,365957,384916,343023,348925 | 339675,383744,343546,370322,384858 | 532 / 1049867 |
| diff-pretty `bench::render_log_000_color` | 408629,427247,420583,401591,392069 | 420185,438831,400533,446722,414989 | 535 / 1102088 |
| diff-pretty `bench::render_plain_unified` | 1278,1456,1352,1227,1208 | 1391,1454,1224,1218,1304 | 26 / 2743 |
| diff-pretty `bench::render_reader_document_synthetic_git_log_10000` | 10543184,10863550,11133612,10055669,10759762 | 10697539,10994200,10776796,10959675,10960369 | 10008 / 705971 |
| diff-pretty `bench::render_reader_to_log_000` | 353351,391367,369208,354844,359983 | 359223,415396,401510,365183,383835 | 561 / 1086041 |
| diff-pretty `bench::render_show_corpus` | 4996050,5179537,5429577,4900664,4950758 | 5079291,5276414,5265013,5243782,5213602 | 6970 / 15174210 |
| diff-pretty `bench::render_show_large` | 1548900,1627620,1748314,1535616,1530075 | 1526548,1646352,1611312,1715354,1598869 | 1542 / 5880164 |
| diff-pretty `bench::render_show_metadata` | 323739,364427,358029,336560,333283 | 328044,350756,345603,340224,362002 | 955 / 772733 |
| diff-pretty `bench::render_show_typical` | 2916923,2982048,3133171,2931591,2856689 | 2853058,3098016,3080570,3119354,3029691 | 4473 / 8521313 |
| scrl `bench::cached_search_redraw` | 10152,10237,9456,8537,9131 | 10248,9498,9171,8777,10404 | 1 / 120 |
| scrl `bench::filtered_stream_and_redraw_500k` | 1888989,1763697,1884581,1885421,1755159 | 1779406,1853087,1932812,1724470,1737952 | 169 / 20130 |
| scrl `bench::filtered_stream_and_redraw_50k` | 197425,181164,183948,189541,170177 | 175777,186708,188779,170790,169521 | 52 / 13970 |
| scrl `bench::filtered_stream_and_redraw_5m` | 19092773,18251475,18960450,19219866,17914485 | 19294241,19971173,19640325,18004263,18960784 | 1335 / 82230 |
| scrl `bench::initial_regex_search_late_match` | 2565375,2774964,2792852,2734419,2529692 | 2583387,2581877,2585785,2428584,2394606 | 105 / 2085525 |
| scrl `bench::initial_search_late_match` | 2613223,2626198,2556835,2702714,2500859 | 2567591,2611754,2580187,2339984,2379935 | 62 / 2084795 |
| scrl `bench::live_search_chunk_and_redraw` | 1321862,1335223,1290808,1384141,1257896 | 1533612,1297923,1424556,1199251,1203210 | 62 / 1054795 |
| scrl `bench::session_redraw_without_movement` | 15408,12900,13271,11516,12116 | 12862,12421,14252,14085,11725 | 1 / 120 |
| scrl `bench::viewport_redraw_horizontal_clip` | 11987,11719,11937,10473,12160 | 11383,11596,11141,11116,10150 | 1 / 120 |
| scrl `bench::viewport_redraw_no_search` | 13154,13323,14781,12731,12848 | 13173,13950,12304,12764,11721 | 1 / 120 |
| scrl `bench::viewport_redraw_styled` | 9816,9673,8910,8506,9866 | 9531,10127,8966,9423,9302 | 1 / 120 |

The five redirected maximum-RSS observations in bytes were: `diff-pretty` baseline 4734976, 4734976, 4767744, 4702208, 4718592; candidate 4685824, 4603904, 4636672, 4603904, 4636672. For `scrl`, all ten observations were 2064384. Each invocation used `/usr/bin/time -l PROGRAM --no-pager`, the concatenated corpus as stdin, a captured stdout, and a 30-second process deadline. All exited 0, and each program's outputs matched across sides.

The five PTY maximum-RSS observations in bytes were: `diff-pretty` baseline 2588672, 2605056, 2605056, 2605056, 2588672; candidate 2588672 in all five. `scrl` baseline was 2293760, 2293760, 2293760, 2293760, 2310144; candidate was 2293760, 2310144, 2293760, 2293760, 2293760. Each run used a 24 by 100 PTY, waited for the first frame, sent `q`, and exited 0. The `diff-pretty` frame was 3304 bytes with SHA-256 `e764b1e723b87a03e1d4aa487c4507f74b0f384ff915ed64df1328f4b5a553b2`; the `scrl` frame was 1850 bytes with SHA-256 `628a9366d2d25e3d2ae5ece4f0dbb144106febf63507b9376ee78f426b2cb646`. Frames matched on both sides. These are early-exit interactive samples, not full traversal or memory reduction claims.

The baseline and candidate `diff-pretty` binary SHA-256 values are `9c30e2d42964bf64687749201ef7d8e3f6ac7ae12c9015e818c32a4c065a7381` and `a0a671a286b67e26ba35e0c26d29244b806ac6b7499f7dfe67f75c2343079cb4`. Both `scrl` binaries have SHA-256 `1acd7f5ad0d05b18e6357e46bfac9aba53a71b5d668735f0bf119aa3026a817f`.

## Reproduce the measurements

Build each tree using the shipping flags in the previous section, setting `CARGO_TARGET_DIR` to the absolute `perf-base` or `perf-candidate` directory under this checkout's `.work`. Build each package's benchmark target using the `--no-run` command above. Set `PATH=/Users/josh/d/stuff/.work/tools/bin:$PATH`, `TMPDIR` to this checkout's `.work/tmp`, `RUSTC_WRAPPER=kache`, `CARGO_BUILD_JOBS=1`, and `CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=clang`. Run `cat fixtures/*.patch > .work/tmp/alignment-corpus.patch`.

Run this pair script from the candidate checkout. It writes JSON receipts and gate logs under `.work/tmp`.

```zsh
#!/bin/zsh
set -u
root=$PWD
base=/Users/josh/repos/laputa-systems/diff-pretty/.work/wt/ab
for package in diff-pretty scrl; do
 for iteration in 1 2 3 4 5; do
  if (( iteration % 2 )); then sides=(base candidate); else sides=(candidate base); fi
  for side in $sides; do
   if [[ $side == base ]]; then tree=$base; target=$root/.work/perf-base; else tree=$root; target=$root/.work/perf-candidate; fi
   label=${package}-${iteration}-${side}
   cd "$tree" || exit 1
   uptime > "$root/.work/tmp/$label.load"
   sysctl -n hw.ncpu >> "$root/.work/tmp/$label.load"
   perf-gate-ready > "$root/.work/tmp/$label.pre" 2>&1; pre=$?
   if (( pre != 0 )); then print "$label pre=$pre"; exit 50; fi
   env PATH=/Users/josh/d/stuff/.work/tools/bin:$PATH CARGO_TARGET_DIR="$target" TMPDIR="$root/.work/tmp" RUSTC_WRAPPER=kache CARGO_BUILD_JOBS=1 CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=clang RUSTFLAGS='-Zlocation-detail=none -Zunstable-options -Cpanic=immediate-abort -Clink-arg=-Wl,-dead_strip_dylibs' cpu-guard --cores 2 -- rustybench baseline --baseline "$root/.work/tmp/$label.json" --warmup 1 --runs 1 --sample-count 15 --sample-size 20 -- cargo bench -p "$package" --bench bench --profile dist -Z build-std=std -Z build-std-features= -Z panic-abort-tests --target aarch64-apple-darwin > "$root/.work/tmp/$label.log" 2>&1
   run=$?
   perf-gate-ready -1 > "$root/.work/tmp/$label.post" 2>&1; post=$?
   print "$label pre=$pre run=$run post=$post"
   if (( post != 0 )); then exit 51; fi
   if (( run != 0 )); then exit 52; fi
  done
 done
done
```

For each iteration one through five, rotate the side order as in the pair script. For each side and each program, run `/usr/bin/time -l` with `--no-pager`, the corpus as stdin, and captured output. Enforce a 30-second deadline. Compare output length and SHA-256 and record `maximum resident set size`.

For the PTY samples, save the following as a Python script and invoke it once per side and program with the binary path and corpus path. Rotate sides as above. Each invocation has an outer 20-second deadline.

```python
import fcntl, hashlib, os, pty, resource, select, signal, struct, subprocess, sys, termios, time
program, corpus = sys.argv[1:]
master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 100, 0, 0))
def setup():
    os.setsid()
    fcntl.ioctl(0 if os.path.basename(program) == 'scrl' else 1, termios.TIOCSCTTY, 0)
source = open(corpus, 'rb') if os.path.basename(program) == 'diff-pretty' else slave
args = [program, '--paging=always'] + ([] if os.path.basename(program) == 'diff-pretty' else [corpus])
p = subprocess.Popen(args, stdin=source, stdout=slave, stderr=slave, preexec_fn=setup, close_fds=True)
os.close(slave)
buf = bytearray()
deadline = time.monotonic() + 10
sent = False
while time.monotonic() < deadline and p.poll() is None:
    ready, _, _ = select.select([master], [], [], .1)
    if ready:
        try: data = os.read(master, 65536)
        except OSError: break
        buf.extend(data)
        if not sent and b'\x1b[' in buf and len(buf) > 100:
            os.write(master, b'q')
            sent = True
try: code = p.wait(timeout=2)
except subprocess.TimeoutExpired:
    p.kill(); p.wait(); code = 124
while select.select([master], [], [], 0)[0]:
    try:
        data = os.read(master, 65536)
        if not data: break
        buf.extend(data)
    except OSError: break
print('exit=%s sent_q=%s bytes=%s sha256=%s rss=%s' % (code, sent, len(buf), hashlib.sha256(buf).hexdigest(), resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss))
sys.exit(0 if code == 0 and sent else 1)

```

Compute the threshold and check receipt shape with this script from the candidate checkout. An assertion failure or a printed regression is not a pass.

```python
import json
import pathlib
import statistics
root = pathlib.Path('.work/tmp')
for package in ('diff-pretty', 'scrl'):
    sides = {}
    for side in ('base', 'candidate'):
        suites = [json.loads((root / f'{package}-{i}-{side}.json').read_text())
                  for i in range(1, 6)]
        assert all(s['schema'] == 1 and s['sample_count'] == 15
                   and s['sample_size'] == 20 and s['warmup_runs'] == 1
                   and s['measured_runs'] == 1 for s in suites)
        names = [{b['name'] for b in s['benchmarks']} for s in suites]
        assert len(names[0]) == 11 and all(n == names[0] for n in names)
        sides[side] = {n: [next(b for b in s['benchmarks'] if b['name'] == n)
                           for s in suites] for n in names[0]}
    assert sides['base'].keys() == sides['candidate'].keys()
    for name in sorted(sides['base']):
        a = sides['base'][name]
        b = sides['candidate'][name]
        x = [r['median_ns'] for r in a]
        y = [r['median_ns'] for r in b]
        delta = statistics.median(y) - statistics.median(x)
        spread = max(max(x) - min(x), max(y) - min(y))
        regression = delta > statistics.median(x) * .02 and delta > spread
        allocations = [[(r['alloc_count'], r['alloc_bytes']) for r in v]
                       for v in (a, b)]
        print(package, name, x, y, 'regression', regression, 'alloc', allocations)
```

This corpus comparison recreates the 103 individual checks. Supply the absolute paths of the two built `diff-pretty` binaries as its arguments.

```python
import pathlib, subprocess, sys
files = sorted(pathlib.Path('fixtures').glob('*.patch'))
assert len(files) == 103
for path in files:
    results = [subprocess.run([binary, '--no-pager'], input=path.read_bytes(),
                              capture_output=True, timeout=30)
               for binary in sys.argv[1:]]
    assert len(results) == 2
    assert all(r.returncode == 0 and not r.stderr for r in results), path
    assert results[0].stdout == results[1].stdout, path
print('103 identical fixture outputs')
```
