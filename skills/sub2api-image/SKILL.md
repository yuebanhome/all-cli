---
name: sub2api-image
description: Use when generating or editing raster images through the sub2api-image CLI, configuring its gpt-image-2 compatible gateway, installing the binary, or troubleshooting its inputs, outputs, config, and exit codes.
---

# sub2api-image

## Overview

Use `sub2api-image` as a command-line client for gpt-image-2 compatible image generation and image editing gateways. It supports text-to-image, whole-image edits, and masked edits, and expects the gateway to return `b64_json` image data.

## Install

Prefer the repository installer for released binaries:

```bash
curl -fsSL https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.sh \
  | bash -s -- --cli sub2api-image
```

Windows PowerShell:

```powershell
iwr -useb https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.ps1 -OutFile $env:TEMP\install.ps1
& $env:TEMP\install.ps1 -Cli sub2api-image
```

From a local checkout:

```bash
cd sub2api-image
cargo install --path .
```

## Configure

Initialize the config, then edit credentials locally:

```bash
sub2api-image --init
```

Default config path is `~/.sub2api-image/config.toml`.

```toml
base_url = "https://your-gateway.example.com"
api_key  = "sk-xxx"

[defaults]
model   = "gpt-image-2"
size    = "1024x1024"
quality = "auto"
```

Useful environment overrides:

- `SUB2API_IMAGE_CONFIG`: absolute path to a config file.
- `SUB2API_IMAGE_HOME`: directory containing `config.toml`.

Do not print or commit real API keys.

## Generate Images

Text-to-image:

```bash
sub2api-image --prompt "a red apple on a white table" -o apple.png
```

Override defaults per request:

```bash
sub2api-image --prompt "minimal ink landscape" \
  --size 1024x1024 --quality high --model gpt-image-2 -o landscape.png
```

## Edit Images

Whole-image edit:

```bash
sub2api-image --image origin.png \
  --prompt "make it watercolor" -o edited.png
```

Masked edit:

```bash
sub2api-image --image origin.png --mask mask.png \
  --prompt "replace sky with starry night" -o edited.png
```

For masks, transparent areas (`alpha=0`) are the regions to redraw.

## Troubleshooting

- Missing config or invalid input exits with code `2`; run `sub2api-image --init` and check `--prompt`, `--output`, and file paths.
- Network failures exit with code `3`; verify `base_url`, connectivity, and gateway availability.
- Gateway 4xx/5xx responses exit with code `4`; inspect stderr unless `--quiet` is set.
- Response parse failures exit with code `5`; this CLI rejects URL-only responses and requires `b64_json`.
- IO failures exit with code `6`; check output directory permissions and input file readability.

## Development

When changing the CLI from a checkout, run:

```bash
cd sub2api-image
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```
