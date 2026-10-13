# Input support

`diff-pretty` reads patch streams from stdin. It handles Git commit headers,
ordinary Git file headers, unified hunks, additions, deletions, renames,
binary-file notices, multi-commit streams, and plain `diff -u` input.

Within recognized diffs, valid SGR color sequences are removed before parsing
and the content is rendered with diff-pretty's styles. Other terminal controls
are removed from diff content. Leading metadata that is not recognized as a
diff keeps its ordinary text, while terminal controls are removed. File-header
metadata that does not render as a hunk or decoration is ignored.

These modes are not rendered as diffs:

- `git blame` output;
- `git grep` or ripgrep output;
- combined or merge diffs; and
- submodule logs or diffs.

The renderer does not infer unsupported formats from plain text. See [output format](output.md) for styling and layout behavior.
