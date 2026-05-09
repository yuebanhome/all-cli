# data-explorer

Use this template when the user wants to filter, sort, and inspect a small dataset
(usually 10-1000 rows) interactively to extract a query or rule.

## Required HTML structure

- Top: filter bar (text search + faceted checkboxes / range sliders).
- Middle: result table or card grid with sortable columns.
- Bottom: `<pre id="prompt">` describing the user's selection criteria + copy button.

## Data source

- Inline the dataset as a `const DATA = [...]` JSON literal in `<script>`. No fetch.
- For datasets > 1000 rows, sample down before inlining.

## Live update

- Each filter event recomputes the visible rows and updates a count: "12 of 487 rows".
- The prompt block always describes the *current* filter state, not the row count.

## Anti-patterns

- Async data load that requires a backend.
- Silent zero-result state with no explanation.
- Prompt that emits the entire result set instead of the rule.
