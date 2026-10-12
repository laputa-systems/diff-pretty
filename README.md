# diff-pretty

A renderer and built-in pager for Git-style unified diffs. It reads patch text
from stdin, restyles it with a fixed presentation (line-number gutters, word-level
highlights, file and hunk decorations), and writes ANSI output to stdout. There is
no configuration file.

```sh
git show | diff-pretty
diff-pretty --paging=never < fixtures/show_003.patch
```

Paging flags: `--paging=auto|always|never` and `--no-pager`. The default is
`always`, which uses the built-in pager only when stdout is a terminal. The pager
is also available as the standalone `scrl` binary (`make scrl`).

## Build

```sh
make release   # static release build
make install   # release build, then copy to ~/usr/bin
make test
```

See `AGENTS.md` for the output contract and repository layout.

## License

MIT
