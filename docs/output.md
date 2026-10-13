# Output format

The renderer uses fixed styles and does not read terminal dimensions or configuration. File-decoration underlines are 80 columns wide. Hunk content remains unwrapped and can exceed 80 columns.

Each hunk line has old and new line-number cells, each at least four columns
wide. Larger line-number ranges use wider cells. The cells have blue borders.
Context numbers are gray, removed numbers dark red, and added numbers dark
green. Removed and added content is red and green; changed word runs use bold
reverse video over the content color. Word runs consist of Unicode
alphanumeric characters and `_`.

Tabs in hunk content expand to eight spaces. Trailing whitespace on added lines uses the whitespace-error style. File decorations use a blue `Δ` label for ordinary changes, or `added:`, `removed:`, or `renamed:` when applicable. Hunk headers are preceded by a blank line. A header with a function fragment gets a blue box containing `• <fragment>`; one without a fragment has no box.

Adjacent ANSI segments add only attributes that can be safely added. Removing a foreground color or attribute emits a reset followed by the complete next style. Styled lines end with one reset.

The pager clips long lines at the terminal edge. See [pager integration](pager.md) for how rendering reaches the terminal session.
