# playctl

Local HTTP playground server controller used by the `live-playground` Claude Code skill.

## Install

Linux / macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.sh \
  | bash -s -- --cli playctl
```

Windows PowerShell (limited; see Caveats):

```powershell
iwr -useb https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.ps1 -OutFile $env:TEMP\install.ps1
& $env:TEMP\install.ps1 -Cli playctl
```

From a local checkout:

```bash
cd playctl
cargo install --path .
```

## Usage

```text
playctl start                # idempotent start, prints URL
playctl stop                 # graceful shutdown
playctl status               # running/stopped + URL + count
playctl list                 # tabular: slug, title, URL, created
playctl new <slug> [--template <name>] [--title "..."] [--description "..."]
playctl reindex              # rebuild .playgrounds/index.json from disk
playctl open [<slug>]        # open URL in default browser
playctl print-template <name>  # print embedded template markdown
```

Global flags: `--root <PATH>`, `--port <N>` (default 4747; +15 retry on collision), `--quiet`, `--json`.

## Layout

`playctl` writes to a single per-project directory:

```
<project_root>/.playgrounds/
├── .runtime/{server.pid, server.port, server.log}
├── index.json
└── <slug>/index.html
```

A first `start` in a git repository appends `.playgrounds/` to `.gitignore`.

## Exit codes

| code | meaning |
|---|---|
| 0 | success |
| 1 | user error (bad args, slug exists / invalid, unknown template) |
| 2 | env error (cannot write `.runtime/`, no project root and no `--root`) |
| 3 | port exhaustion (16 consecutive ports occupied) |
| 4 | server failed to start within 5s |
| 5 | internal error |

## Templates

The six embedded templates are read by Claude Code via `playctl print-template <name>`:

`design-playground`, `data-explorer`, `concept-map`, `document-critique`, `diff-review`, `code-map`.

## Caveats

- Native Windows is **not supported** in v1. The `start`/`stop`/`status` commands bail with a clear message; use WSL2.
- The HTTP server binds to `127.0.0.1` only and provides no auth — never expose externally.
- Port selection has a small TOCTOU window: `start` probes a free port, drops the listener, then spawns the child to re-bind. Another local process can grab the port in that window, in which case the 5s `/healthz` wait times out and `start` exits 4. Re-run.
- Static files under `.playgrounds/<slug>/` are served with `tower-http`'s `ServeDir`; the handler also runs a `canonicalize` check to keep symlinks from escaping the playground directory. Don't put untrusted symlinks under `.playgrounds/`.

## Development

```bash
cd playctl
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```
