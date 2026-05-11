# concept-map

Use this template when the user is exploring how concepts in a domain relate, and
wants to mark which they understand / care about / want to dive into.

## Required HTML structure

- One node per concept, rendered as a card with: title, 1-line summary, status pill.
- Click a card to cycle status: `unknown → learning → solid → dive-deeper`.
- Connecting lines drawn in SVG between related concepts.

## Live update

- The prompt block summarizes: "I'm solid on X, Y. Currently learning Z. Want to go deeper on W." Construct it from the per-card status.

## Anti-patterns

- Force-directed graph (overengineered; use a static layout).
- Hidden statuses behind a menu — surface them on the card itself.
