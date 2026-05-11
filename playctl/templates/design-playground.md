# design-playground

Use this template when the user wants to tune visual / spatial design parameters
(radius, shadow, spacing, color, typography) and see the result update live.

## Required HTML structure

- Single file (`index.html`), no external resources.
- Dark theme by default, but honor `prefers-color-scheme: light`.
- Two-column layout on viewports ≥ 800px:
  - Left: `<form>` with one labelled `<input>` per parameter.
  - Right: `<div id="preview">` with the artifact being designed.
- Below the columns: a `<pre id="prompt">` block followed by a copy button.

## Controls

- Always provide presets (3-5 named buttons that set all controls at once).
- All numeric controls use `<input type="range">` paired with a numeric display.
- Color uses `<input type="color">`.

## Live update

- Wire control changes to a single `update()` function that reads all values, mutates the preview, and rewrites the prompt block.
- Run `update()` once on load.

## Prompt output

- The prompt block is a complete sentence (or short paragraph) the user can paste back into chat.
- Avoid value dumps. Translate values to design intent: "soft shadow, generous corner radius" — not "shadow=12px, radius=24px".
- Include a copy button that reads from the `<pre>` and writes to clipboard, with a 2-second "copied!" feedback.

## Anti-patterns

- External CDN (use system font stack only).
- No presets.
- Prompt = key=value list of all sliders.
- Preview only updates on submit.
