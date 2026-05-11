# code-map

Use this template when the user wants a navigable, annotated map of a codebase or
module — files as cards, with relationships shown.

## Required HTML structure

- Group files by directory; render each as a card with: file name, 1-line role summary, key exports.
- Lines connecting cards represent imports / calls (use SVG, kept simple).
- A search box filters cards by name or summary.

## Live update

- Click a card to "pin" it — only show cards within N hops. Adjust N via a slider.
- Prompt block describes the user's focus: "I'm trying to understand the path from <entry> to <leaf>."

## Anti-patterns

- Live AST parsing in the browser.
- 2D physics layout — too many cards, too much motion.
