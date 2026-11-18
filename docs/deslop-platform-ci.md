# Whole-change mutation gate

Status: local implementation under qualification; remote Linux/macOS CI and the
parent's mixed negative-control PR have not been run. The lane must not be
called qualified until each required component completes and its result is
reconciled. The old whole-workspace run remains incomplete.

`make mutants-diff BASE=<target-ref> SHARD=<component>` compares the merge
base with the working tree, reports every changed path and class, adds changed
production mutations to the critical selection for tests, fixtures, build or
gate changes, and prints the complete inventory before running one component.
A documentation-only change has no mutation selection. The six disjoint
components are `cli`, `config`, `render`, `source`, `session` and `other`;
the last receives production files outside the named components. Each job
recomputes the whole selection and checks that its partitions cover it.
A missing result, unexpected empty relevant selection, unknown path, unexpected
identity, unresolved timeout or exhausted deadline fails. No cargo-mutants
exclusion config is loaded; each recorded functional equivalence is evaluated
only after the mutant runs and only at its named source hash. The renderer uses
the CLI target because its owned child bounds sanitizer hangs; other components
run the workspace caller tests. Each component has a 1,200-second budget,
60-second test timeout and 300-second build timeout. A timeout is retried with
a fresh baseline and three times the test timeout; only a separately proved,
source-pinned hang may be treated as detected. Receipts remain in ignored
`.work/mutants-gate/` and stdout names every selected identity and result.

The source script and its tests are adapted from srv's gate; their ISC notice
is preserved in each file. The alternative of one whole-workspace selection was
not chosen: its historical 1,200-second attempt left 109 unrun and 48 timeouts
unconfirmed. Named components retain the qualified CLI, numeric, sanitizer,
source and session caller scopes while accepting additional changed production
mutants rather than treating the critical scope as a fallback.

The CI matrix names both existing runner platforms and all six components.
It uses the fleet-approved `actions/setup-python` v7.1.0 and
`taiki-e/install-action` v2.90.4 pinned commits, with cargo-mutants 27.1.0 in
an ephemeral `CARGO_HOME`; no production dependency or shipped build changes.
The installed local tool is Mach-O arm64, SHA-256
`1da2abafde0e0fedee3939043859a01456650767557d22fd8ab77b30cee96719`,
linked to macOS system libraries. Its published crate declares MIT and Rust
1.88; its lockfile SHA-256 is
`363f0406f610ee408f22171a53ed17e88d0989d1f50b680cb0f5cbc0711e1902`.
That lock has 203 entries; 169 locally available manifests reported permissive
license expressions, while 34 were unavailable locally. This is not a complete
transitive license or downloaded-CI-artifact verification; the parent must
resolve that before qualification. The tool is development-only and the
project Cargo.lock and manifests are unchanged.

## Local evidence in progress

At `6f1719d8f0bb1ff9ab42ea0fcc91994ea9909ccb`, merge base
`aa0e5053aced5644db254acdc0e392a517abb7d2`, macOS arm64, rustc
1.102.0-nightly and cargo-mutants 27.1.0, the gate initially selected 217
identities: CLI 24, config 38, render 80, source 17, session 58, other 0.
The CLI component completed with 22 caught and two compiler-unviable; no
misses or timeouts. The full inventory and results are in ignored
`.work/tmp/cg-gate-cli-5.log`. This run predates final gate edits and is not a
final activation. `make test` and `cargo check --workspace --all-targets`
passed at the same app revision, as did `make lint`.

## Parent negative control

On a throwaway PR, delete `supported_paging_flags_choose_the_requested_mode`
from `src/main.rs` and independently reorder `PagingMode::Always | PagingMode::Auto`
in `src/source.rs`. First verify ordinary checks pass. The gate must print both
the source production selection and the CLI critical selection; its CLI result
must show the `NO_PAGER` comparison inversion missed. A failure only for the
unrelated source change does not prove the union. This blueprint is unexecuted.

The initial renderer activation used workspace tests instead of the qualified
bounded CLI caller. It was interrupted after 30 outcomes (21 caught, four
missed, one unviable and four 60-second timeouts); this is an incomplete,
invalid component measurement and does not confirm the timeouts. Its snapshot
is `.work/tmp/cg-render-invalid-partial` and the outer log is
`.work/tmp/cg-gate-render.log`. The corrected invocation uses the CLI caller.

The corrected renderer run (`.work/tmp/cg-gate-render-cli.log`, receipt
`.work/mutants-gate/run-4vly5ld7`) executed all 80 identities: 71 caught,
eight missed, one unviable, zero timeouts; the gate exited 1 because the eight
known equivalences were not yet configured. These are the same eight explained
in [the sanitizer receipt](deslop-cli-tests.md#terminal-control-rendering).
The extra 80th mutant is the changed `render_chunk` replacement and was caught.
The CLI inventory matches the separate complete bounded CLI receipt's 24 names
exactly (sorted-name SHA-256
`2a4a12b550098478e74ebfdd340325e17aecdd2f410c3c231e716a95bcb87257`).
The source component (`.work/tmp/cg-gate-source.log`) exited 0 with 16 caught,
one unviable, zero misses and zero timeouts across 17 identities.

## Local reproduction

Python 3.11 or newer is required for `tomllib`. In this checkout, prepend
`/opt/homebrew/bin` and `/Users/josh/d/stuff/.work/tools/bin` to `PATH`, set
`TMPDIR=$PWD/.work/tmp`, `CARGO_TARGET_DIR=$PWD/.work/target-cg-gate`,
`CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, `RUSTC_WRAPPER=kache`, and
`PYTHON_CPU_COUNT=2`. Create `TMPDIR` first. `make mutants-gate-test` runs the
semantic selector/result fixtures. For each required component, acquire the
shared atomic mutation lock first and an available heavy slot second; run one
command under `cpu-guard --cores 2` and release only the directories that
invocation created:

```sh
mkdir /Users/josh/d/stuff/.work/tmp/mutation.lock || exit 75
slot=''
for n in 1 2 3; do
  if mkdir /Users/josh/d/stuff/.work/tmp/deslop-heavy-slots/$n 2>/dev/null; then
    slot=$n; break
  fi
done
if [ -z "$slot" ]; then
  rmdir /Users/josh/d/stuff/.work/tmp/mutation.lock; exit 75
fi
trap 'rmdir /Users/josh/d/stuff/.work/tmp/deslop-heavy-slots/$slot; rmdir /Users/josh/d/stuff/.work/tmp/mutation.lock' EXIT
cpu-guard --cores 2 -- python3 -B scripts/mutants_gate.py origin/main --shard "$SHARD"
```

The script's own deadline includes inventory, baseline, mutation and retries.
Do not remove a lock created by another invocation. Full component receipts
include mutant diffs and test logs under the printed receipt directory. CI
must independently establish both platform results and the parent negative
control; local Darwin measurements cannot establish Linux behavior.

The session component (`.work/tmp/cg-gate-session.log`, receipt
`.work/mutants-gate/run-futp1y16`) exited 0 in 174.6 seconds: 56 caught,
one unviable, one source-pinned equivalent and no timeouts, out of 58.
The most recent selftest run passed nine tests. The CLI and source measurements
precede subsequent gate edits; render was measured before its dispositions
were configured. The numeric component, final activation and final aggregate
have not run. Do not treat the implementation as a locally qualified gate.

An offline `cargo tree --locked` attempt for the tool did not resolve: the
configured mirror's local index did not have locked `anyhow 1.0.86` (the output
listed 1.0.104 and 1.0.102). Its outer exit was not captured. This does not
complete transitive or CI artifact vetting. The parent must establish those,
run the remaining local qualification if needed, verify both CI platforms, and
perform the mixed negative control before treating the lane as qualified.
