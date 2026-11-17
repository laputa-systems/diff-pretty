# Root CLI process slice

Base `03d8acec93d7594aeedb5a74a7434b99ac5643fd`; donor
`6f1719d8f0bb1ff9ab42ea0fcc91994ea9909ccb`. This is a test-only
extraction. The original bare-control test and its helper remain intact.
The root CLI suite has eight test functions: invalid paging diagnostic; five
redirected paging forms against one literal rendered diff; directory stdin
error; closed stdout error; the existing two help flags and footer; empty
stdin; the two existing bare-control examples; and extended control-sequence
metadata and hunk bytes. Error messages from OS I/O are checked by prefix,
not by platform-specific error wording. The pure-std helper owns its child,
kills and reaps on unwinding, and bounds completion at five seconds. Its
fixtures fit in pipes; it is not a general-purpose large-output runner.

On macOS, the focused CLI test, full release workspace suite and lint each
exited 0. The focused suite reported eight passed, none failed or ignored.
Each command ran with a separately acquired shared heavy slot and the following
settings, releasing the slot after the command:

```sh
export TMPDIR="$PWD/.work/tmp" CARGO_TARGET_DIR="$PWD/.work/tmp/cli-ordinary-target"
export CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 RUSTC_WRAPPER=kache PYTHON_CPU_COUNT=2
cpu-guard --cores 2 -- cargo test -p diff-pretty --test cli --offline
cpu-guard --cores 2 -- make test
cpu-guard --cores 2 -- make lint
```

Four decisive hand faults each exited 101 on their intended assertion.
The invalid-mode diagnostic changed, redirected output gained a prefix,
I/O errors reported success, and OSC BEL termination was disabled.
The controls ran separately under the shared mutation lock with owned heavy
slots. Both source files were restored byte for byte after every control.
`src/main.rs` SHA-256 was
`48cfccadd5fe79d0a9634c32de571530b0af197e2839454b6daaf2e7381b4af5` and
`src/render.rs` was
`fcaa24c85b951257b53f5ea93eb4f3638e03936194755325af870bfa40918398` after
the same before and after the controls. To reproduce, run the focused CLI
suite in a separate mutation target, restoring both source files between
faults. Change `invalid paging mode: {v}` to `bad paging mode: {v}`;
append `print!("fault");` after `let mut output = stdout.lock();`;
change `std::process::exit(1);` to `std::process::exit(0);`; or change
`bytes[end] == 0x07` to `bytes[end] == 0x06` in `terminal_control_end`.
Each must fail its corresponding output or status assertion. Acquire
the mutation lock before modifying source and a heavy slot for each test;
restore and hash both files before releasing the lock. Repeat the ordinary
checks using a separate target after faults. The parent repeated `make test`
and `make lint` after all four controls. Both exited 0, including all eight
root CLI tests. An earlier lock acquisition was busy and applied no fault.

The donor's owned PTY entry tests remain parked: their root test target needs
the donor's `rustix` dev-dependency, which is outside this slice. Interactive
entry, terminal restoration, early quit and its kill/reap deadline are not
qualified here. The broader Tests and CI work remains open. No runtime or
whole-step performance claim is made and the donor's binary-specific timing
receipt is not transferred.
