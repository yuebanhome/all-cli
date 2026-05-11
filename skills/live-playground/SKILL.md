---
name: live-playground
description: Use when the user wants an interactive, visual tool to explore options, try parameters, or make selections — anything that benefits from a live preview with controls. Builds a single-file HTML playground served at a local URL.
---

# live-playground

## Overview

Use `live-playground` whenever the user asks for a tunable, visual exploration —
"let me try shadow values", "build me a picker for X", "show this with a slider".
This skill drives the `playctl` CLI to spin up a local HTTP server and serve a
self-contained HTML playground per project.

## Install

Linux / macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.sh \
  | bash -s -- --cli playctl
```

Windows PowerShell (limited; lifecycle commands bail — use WSL2 instead):

```powershell
iwr -useb https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.ps1 -OutFile $env:TEMP\install.ps1
& $env:TEMP\install.ps1 -Cli playctl
```

From a local checkout: `cd playctl && cargo install --path .`.

Compatibility: this skill assumes `playctl >= 0.1.0`.

## When to use

- "let me try parameters / play with values for X"
- "build me a picker / chooser / explorer"
- "show me this live as I change it"
- "I want to compare options visually"
- "annotate this document / diff section by section"

## When NOT to use

- One-shot text answers.
- Fixed snippets the user just wants delivered.
- Anything where a single static HTML file would not benefit from controls.

## Workflow

1. **Pre-check**: run `command -v playctl`. If missing, surface the install command above and stop.
2. **Pick a type** (one of):
   `design-playground`, `data-explorer`, `concept-map`, `document-critique`, `diff-review`, `code-map`.
3. **Read the canonical template**:
   ```bash
   playctl print-template <type>
   ```
   Treat the output as binding guidance for the HTML you write.
4. **Pick a slug** matching `[a-z0-9][a-z0-9-]{0,63}`, plus a one-line title and description.
5. **Scaffold + auto-start**:
   ```bash
   playctl new <slug> --template <type> --title "..." --description "..."
   ```
   This creates `<project_root>/.playgrounds/<slug>/index.html`, registers the
   playground in `index.json`, starts the daemon if needed, and prints the URL.
6. **Fill in the HTML**: use the Write tool to overwrite
   `<project_root>/.playgrounds/<slug>/index.html` per the template's structure.
   Single file, no external resources, dark theme by default.
7. **Hand off the URL**: print the URL plus a one-sentence "what to do" hint.
   The user opens it, operates the controls, copies the prompt block, and
   pastes it back into chat.

If `playctl new` does not fit (e.g., reusing an existing slug, or writing
multiple files at once), use the fallback path: write the file directly with
Write, then `playctl reindex && playctl start`.

## Core requirements (for the HTML you write)

- Single file at `.playgrounds/<slug>/index.html`. No CDN, no external fonts.
- Live preview that updates on every control change (no submit button).
- Prompt block (`<pre>` or similar) with a copy button, written as a sentence
  the user pastes back.
- Sensible defaults + 3-5 named presets.
- Dark theme; honor `prefers-color-scheme: light` if trivial.

## Anti-patterns

- External CDN (load order, offline failure).
- "Value-dump" prompt that just lists `key=value` for every slider.
- No presets — the user has to discover sensible starting points.
- Preview that updates only on a button click, not on input.

## Diagnostics

- `command not found: playctl` → reissue the Install command at the top of
  this skill. Do **not** silently `cargo install --git`; the user may not have
  Rust installed.
- `exit 3 (port exhaustion)` → another `playctl` instance is occupying ports
  4747-4762. Ask the user to `playctl stop` in that other project, or pass
  `--port <higher>`.
- `exit 2 (env error)` → user is not in a project directory and did not pass
  `--root`. Suggest `cd` into a project, or `playctl --root /path/to/project ...`.
- `exit 4 (server-not-up)` → child server died during 5s healthz wait. Show
  `<project_root>/.playgrounds/.runtime/server.log` to the user.
- Native Windows: `start`/`stop`/`status` will bail with "use WSL2". File
  operations (`new`, `reindex`, `print-template`) still work.
