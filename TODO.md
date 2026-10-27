Prefix: DIFF
Next: 4

### DIFF-1 · P1 · live pager goes raw before it can quit or decide to page

- **Area:** Concurrency
- **Where:** `scrl/src/session.rs:580`, `scrl/src/session.rs:826`, `scrl/src/session.rs:893`, `scrl/src/session.rs:929`, `scrl/src/session.rs:1059`, `scrl/src/document.rs:108`, `scrl/src/terminal.rs:284`, `scrl/src/main.rs:19`, `src/source.rs:20`
- **Problem:** `run_terminal` always enters `run_terminal_live` for `Always` and `Auto`. That function calls `RawMode::enter` before it knows whether the text fits, then blocks in `receiver.recv` until a screen is full or the source ends. The fill loop never reads keys or `terminated()`, and raw mode clears `ISIG`, so Ctrl-C and SIGTERM do nothing during that wait. `scrl` defaults to `auto`. When the document's line count stays at or below the screen, `write_to` then copies the raw bytes, including bare `\n`, while raw mode is still held (`drop(raw)` is after that write). `Termios::make_raw` clears `OPOST` (rustix 1.1.4 `cfmakeraw`; the libc backend calls the platform `cfmakeraw`). A PTY write of `hello\nworld\n` after `tty.setraw` produced `b'hello\nworld\n'` with no CR, so the terminal receives LF without carriage return. `diff-pretty --paging=auto` uses the same runner. The cooked write that already exists around `session.rs:627` is unreachable for these modes. The running pager was not executed: this tree has no build artifacts.
- **Recommendation:** Leave the terminal cooked until paging starts. During the prefetch, observe quit and SIGTERM instead of blocking forever on the source channel. For `auto` text that fits, call `write_to` only after output post-processing is still on, or restore it first. Keep the alternate-screen path for text that does not fit.
- **Fix with:** lifecycle-concurrency
- **Effort:** S

### DIFF-2 · P3 · a filter rebuilds the whole document on every chunk

- **Area:** Performance
- **Where:** `scrl/src/session.rs:222`
- **Problem:** With `--filter` set, every `push_chunk` appends to the source document and then assigns `self.document` from `source_document.filtered(pattern)`, which walks every line so far. A large stream is quadratic in the number of chunks. The default path appends without rebuilding.
- **Recommendation:** Apply the filter to the newly appended lines and append only the matches to the display document. Rebuild only when the pattern changes.
- **Fix with:** make-it-fast
- **Effort:** M

### DIFF-3 · P3 · SIGTERM restores the terminal and then exits zero

- **Area:** Operability
- **Where:** `scrl/src/terminal.rs:28`, `scrl/src/session.rs:753`, `scrl/src/session.rs:935`, `scrl/src/main.rs:87`, `src/main.rs:39`
- **Problem:** The SIGTERM handler only sets `TERMINATED`. After the event loop is running, both terminal runners break, restore the screen, and return `Ok`. Both `main` functions ignore `ExitReason` and exit 0. The handler is not reset and the signal is not raised again, so the parent does not see signal death. `signal`'s return value is also ignored, so a failed install is silent. Not executed in a running pager.
- **Recommendation:** After the terminal is restored, set SIGTERM back to `SIG_DFL` and re-raise it. If `signal` fails at install, return the error instead of paging with no handler.
- **Fix with:** lifecycle-concurrency
- **Effort:** S
