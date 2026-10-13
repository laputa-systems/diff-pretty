# Pager integration

The `diff-pretty` CLI adapts rendered diff chunks to the generic `scrl` pager.
The renderer owns Git parsing, word-diff pairing, ANSI styles, decorations, and
paging policy. `scrl` owns the retained document, search, viewport, key decoding,
and terminal lifecycle; it does not know about Git or diff layout.

For terminal paging, `src/source.rs` builds `scrl::RunOptions` and supplies a
diff-specific `ChunkSource` to `scrl::run_source`:

```text
patch stdin → diff parser and renderer → complete diff-file chunks → scrl session
```

The adapter keeps each file together because word-diff pairing needs the full
file context. Non-interactive output streams through `render_reader_to`
without retaining the complete ANSI output. For live paging, a worker reads
diff-file chunks through a one-chunk channel, which applies backpressure. The
pager fills the initial viewport, then asks for more as the user navigates.
Search begins in the loaded prefix; moving to another match can request more
input, and `End` or `G` reads through EOF. Loaded content is retained. In
`auto` mode, output that fits the screen is written directly so it stays in
terminal scrollback. When paging is disabled, stdout is redirected, or the
terminal cannot be used, rendering writes directly without alternate-screen
controls.

The embedded session fixes the diff presentation: wrapping, follow mode,
filtering, and file operands are disabled. `scrl` also ships as a standalone
command and library; its user-facing options and API are in
[`scrl/README.md`](../scrl/README.md).
