# document-critique

Use this template when the user wants to walk through a document section by section
and approve / reject / comment on each piece.

## Required HTML structure

- Sections rendered top-to-bottom, each with: heading, body, and three buttons: ✓ approve, ✗ reject, 💬 comment.
- A comment button opens an inline `<textarea>`.
- A summary panel at the bottom: counts (approved/rejected/commented) and the prompt block.

## Live update

- Recompute counts and the prompt on every action.
- Prompt format: "Approved §1, §3. Rejected §2 (reason: …). Comments on §4: …"

## Anti-patterns

- Modal dialog for comments — keep them inline.
- Lose state on reload (local storage is fine but optional).
