# diff-review

Use this template when the user wants to comment on a unified diff, line by line.

## Required HTML structure

- Render diff with monospace font; preserve `+ / - / context` coloring.
- Each `+` / `-` line gets a click target opening an inline comment box.
- Sidebar: list of comments with line refs, click to scroll to the line.

## Live update

- Comment additions update the sidebar and the prompt instantly.
- Prompt format: a list of `file:line` entries each followed by the comment.

## Anti-patterns

- Single textarea collecting all comments (defeats the point).
- Diff parsed at runtime — accept already-parsed structure passed in via `const DIFF = [...]`.
