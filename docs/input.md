# Input support

`diff-pretty` reads patch streams from stdin. It handles Git commit headers,
ordinary Git file headers, unified hunks, additions, deletions, renames,
binary-file notices, multi-commit streams, and plain `diff -u` input.

Within recognized diffs, SGR color sequences are removed before parsing and
the content is rendered with diff-pretty's styles. Leading metadata that is
not recognized as a diff is passed through unchanged apart from line-ending
handling. File-header metadata that does not render as a hunk or decoration is
ignored.

These modes are not rendered as diffs:

- `git blame` output;
- `git grep` or ripgrep output;
- combined or merge diffs; and
- submodule logs or diffs.

The renderer does not infer unsupported formats from plain text. See [output format](output.md) for styling and layout behavior.
