# diff-pretty

`diff-pretty` reads Git-style patches from stdin and renders colored line
numbers, word changes, and file decorations. It can show long output in its
built-in pager. It has no configuration file and never invokes an external
pager.

## Install

Install the current repository version with Cargo (requires Rust):

```sh
cargo install --git https://github.com/laputa-systems/diff-pretty --package diff-pretty
```

The manual release workflow publishes prebuilt Linux musl binaries as
pre-releases on the [GitHub releases](https://github.com/laputa-systems/diff-pretty/releases)
page.

## Use

```sh
git show | diff-pretty
diff-pretty --paging=never < changes.patch
```

Paging options are `--paging=auto`, `--paging=always`, and `--paging=never`;
`--no-pager` is an alias for `never`. The default is `always`: an interactive
terminal opens the built-in pager, while redirected output is written directly.
`auto` opens the pager only when terminal output exceeds one screen. Any other
`--paging` value is an error.

The renderer uses an 80-column file underline. Hunk content is not wrapped or
clipped, so long lines can exceed the terminal width; the pager clips them at
the terminal edge. See [supported input](docs/input.md) for the patch formats
and cases it handles.

## Help and license

Report issues at [GitHub Issues](https://github.com/laputa-systems/diff-pretty/issues). Licensed under [MIT](LICENSE).
