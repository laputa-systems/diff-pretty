# Alignment boundary evidence

Status: incomplete. DSLP-9 remains open. The required workspace Clippy check fails in unchanged `scrl/src/lib.rs:165` on both baseline and candidate. The assigned scope does not permit editing that file. Timing, RSS, and complete acceptance are unverified.

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

The five matched shipping-profile benchmark pairs, 22 timing rows, allocation comparison, redirected and PTY RSS samples, and host gates were not run. No timing or memory conclusion is supported. The workspace Clippy failure requires an out-of-scope correction before acceptance can be established.
