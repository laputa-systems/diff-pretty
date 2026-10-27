Prefix: DIFF
Next: 4

### DIFF-3 · P3 · SIGTERM restores the terminal and then exits zero

- **Area:** Operability
- **Where:** `scrl/src/terminal.rs:28`, `scrl/src/session.rs:753`, `scrl/src/session.rs:935`, `scrl/src/main.rs:87`, `src/main.rs:39`
- **Problem:** The SIGTERM handler only sets `TERMINATED`. After the event loop is running, both terminal runners break, restore the screen, and return `Ok`. Both `main` functions ignore `ExitReason` and exit 0. The handler is not reset and the signal is not raised again, so the parent does not see signal death. `signal`'s return value is also ignored, so a failed install is silent. Not executed in a running pager.
- **Recommendation:** After the terminal is restored, set SIGTERM back to `SIG_DFL` and re-raise it. If `signal` fails at install, return the error instead of paging with no handler.
- **Fix with:** lifecycle-concurrency
- **Effort:** S
