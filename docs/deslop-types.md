# Filtered session qualification

`FilteredState` keeps a session's filter and retained source together. The
private representation removes six invariant `expect` calls; public APIs,
rendered output and file formats are unchanged. A compile witness assigning
`None` to its required `Document` fails with E0308. Workspace tests and lint
pass on macOS arm64.

The baseline is `c0a81b8`; the candidate runtime is `e4743b5`. Both use the
same lockfile, pinned nightly, default features and optimized Cargo bench
profile. The identical added workload file has SHA-256
`ef6a1dc3dec6b14be8162f1e2042027bae730ee1ac60eb4fed593ec9dd3bd5f7`.

Five baseline/candidate pairs alternate in that order for each suite. Each
side uses `rustybench baseline --warmup 1 --runs 1 --sample-count 5
--sample-size 10 -- cargo bench -p PACKAGE --bench bench`, with PACKAGE
`diff-pretty` or `scrl`. Each side passes `perf-gate-ready` before and
`perf-gate-ready -1` afterward, under `cpu-guard --cores 6`. The benchmark
library is rustybench 0.5.0. Renderer and pager results are collected
separately: the CLI collector retains only the last suite from a multi-suite
Cargo invocation, so that invocation alone does not establish renderer evidence.

The existing pager workloads never enable filtering. Three added workflows
stream sorted real Git-diff fixtures through a filtered session in UTF-8-safe
4-KiB chunks, finish it, navigate to the bottom and draw a real frame. Setup
is outside the measured closure; session construction and retained source
cost are inside. Their inputs are 53,133, 530,646 and 5,306,460 bytes.
The selection oracle uses visible document lines, including ANSI removal,
and preserves the document's final empty row. Both revisions satisfy it.

Each timing cell is the median of five run medians, with their range, in ms.

| Filtered workflow | Before | After | Reported allocations before / after |
| --- | ---: | ---: | ---: |
| 50 KiB minimum | 0.174 (0.172–0.187) | 0.174 (0.174–0.176) | 52 / 52 |
| 500 KiB minimum | 1.780 (1.747–1.874) | 1.767 (1.754–1.802) | 169 / 169 |
| 5 MiB minimum | 18.175 (18.089–18.838) | 18.241 (18.198–18.757) | 1,335 / 1,335 |

Across all 22 renderer and pager rows, no slowdown exceeds both 2% and the
larger observed per-side range. Reported allocation counts and allocated
bytes are unchanged. [The complete comparison](deslop-types-performance.json)
retains every row. No general speedup is claimed.

Five interleaved whole-pager benchmark executions under `/usr/bin/time -l`
give peak RSS medians 47,955,968 bytes before and 48,136,192 after; ranges
47,955,968–48,283,648 and 48,119,808–48,349,184 overlap. Release binary
sizes are unchanged: diff-pretty 853,800 bytes and scrl 816,344 bytes.
These memory checks are separate from the allocator counters.
