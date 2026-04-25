#!/usr/bin/env bash
# Cross-CLI installer for all-cli releases (Linux / macOS).
# Downloads a prebuilt binary from GitHub Releases and verifies SHA256.
set -euo pipefail

REPO="${REPO:-yuebanhome/all-cli}"
CLI_NAME="${CLI_NAME:-}"
VERSION="${VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
TARGET_OVERRIDE="${TARGET_OVERRIDE:-}"
DRY_RUN="${DRY_RUN:-false}"

usage() {
  cat <<'EOF'
Usage:
  install.sh --cli <name> [--version <vX.Y.Z>] [--dir <path>] [--dry-run]

Env vars (flags override these when both set):
  REPO            default: yuebanhome/all-cli
  CLI_NAME        equivalent to --cli
  VERSION         equivalent to --version (default: latest)
  INSTALL_DIR     equivalent to --dir (default: ~/.local/bin)
  TARGET_OVERRIDE force a specific Rust target triple

Exit codes:
  0 success | 1 generic | 2 unsupported platform | 3 download failure | 4 hash mismatch
EOF
}

while (( $# > 0 )); do
  case "$1" in
    --cli)     CLI_NAME="$2"; shift 2 ;;
    --version) VERSION="$2"; shift 2 ;;
    --dir)     INSTALL_DIR="$2"; shift 2 ;;
    --dry-run) DRY_RUN=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
done

if [[ -z "$CLI_NAME" ]]; then
  echo "--cli is required" >&2; usage >&2; exit 1
fi

if command -v curl >/dev/null 2>&1; then
  _dl()    { curl -fsSL "$1" -o "$2"; }
  _dl_out(){ curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
  _dl()    { wget -q "$1" -O "$2"; }
  _dl_out(){ wget -q "$1" -O -; }
else
  echo "Need curl or wget" >&2; exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  _sha() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
  _sha() { shasum -a 256 "$1" | awk '{print $1}'; }
else
  echo "Need sha256sum or shasum" >&2; exit 1
fi

detect_target() {
  if [[ -n "$TARGET_OVERRIDE" ]]; then
    echo "$TARGET_OVERRIDE"; return
  fi
  local s m
  s=$(uname -s); m=$(uname -m)
  case "$s/$m" in
    Linux/x86_64)              echo "x86_64-unknown-linux-musl" ;;
    Linux/aarch64|Linux/arm64) echo "aarch64-unknown-linux-musl" ;;
    Darwin/arm64)              echo "aarch64-apple-darwin" ;;
    *) echo "Unsupported platform: $s $m" >&2; exit 2 ;;
  esac
}

resolve_version() {
  if [[ "$VERSION" != "latest" ]]; then
    echo "$VERSION"; return
  fi
  local api="https://api.github.com/repos/${REPO}/releases?per_page=100"
  local body tag
  if ! body=$(_dl_out "$api"); then
    echo "Failed to query GitHub API at ${api} (network error or rate limit)" >&2
    exit 3
  fi
  # All tags starting with <cli>-v...
  # Strip pre-release: SemVer pre-releases have a '-' inside the version part, i.e.
  #   sub2api-image-v0.1.0       → keep   (version = v0.1.0, no hyphen after v)
  #   sub2api-image-v0.2.0-rc.1  → drop   (version = v0.2.0-rc.1, has hyphen)
  tag=$(printf '%s' "$body" \
    | grep -o '"tag_name": *"[^"]*"' \
    | sed 's/"tag_name": *"\(.*\)"/\1/' \
    | grep "^${CLI_NAME}-v" \
    | awk -v p="${CLI_NAME}-v" '
        { v = substr($0, length(p)+1); if (index(v, "-") == 0) print }
      ' \
    | head -1 || true)
  if [[ -z "$tag" ]]; then
    echo "No release tagged ${CLI_NAME}-v* found in ${REPO}" >&2; exit 3
  fi
  echo "${tag#${CLI_NAME}-}"
}

main() {
  local target version archive url sums_url tmp expected actual
  target=$(detect_target)
  version=$(resolve_version)
  case "$version" in v*) ;; *) version="v${version}" ;; esac
  archive="${CLI_NAME}-${version}-${target}.tar.gz"
  url="https://github.com/${REPO}/releases/download/${CLI_NAME}-${version}/${archive}"
  sums_url="https://github.com/${REPO}/releases/download/${CLI_NAME}-${version}/SHA256SUMS"

  echo "CLI       : ${CLI_NAME}"
  echo "Version   : ${version}"
  echo "Target    : ${target}"
  echo "Archive   : ${url}"
  echo "InstallTo : ${INSTALL_DIR}"

  if [[ "$DRY_RUN" == "true" ]]; then
    echo "(dry run, not downloading)"; return 0
  fi

  tmp=$(mktemp -d); trap "rm -rf '$tmp'" EXIT

  echo "Downloading archive..."
  _dl "$url" "$tmp/$archive" || { echo "Download failed: $url" >&2; exit 3; }

  echo "Downloading SHA256SUMS..."
  _dl "$sums_url" "$tmp/SHA256SUMS" || { echo "Download failed: $sums_url" >&2; exit 3; }

  expected=$(awk -v a="$archive" '$2 == a { print $1; exit }' "$tmp/SHA256SUMS")
  if [[ -z "$expected" ]]; then
    echo "SHA256SUMS missing entry for ${archive}" >&2; exit 4
  fi
  actual=$(_sha "$tmp/$archive")
  if [[ "$expected" != "$actual" ]]; then
    echo "SHA256 mismatch for ${archive}" >&2
    echo "  expected: ${expected}" >&2
    echo "  actual  : ${actual}" >&2
    exit 4
  fi
  echo "SHA256 OK"

  tar -xzf "$tmp/$archive" -C "$tmp"
  mkdir -p "$INSTALL_DIR"
  mv "$tmp/$CLI_NAME" "$INSTALL_DIR/$CLI_NAME"
  chmod +x "$INSTALL_DIR/$CLI_NAME"
  echo "Installed: ${INSTALL_DIR}/${CLI_NAME}"

  case ":${PATH:-}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      echo
      echo "Note: ${INSTALL_DIR} is not on PATH. Add it via:"
      echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
      ;;
  esac

  "${INSTALL_DIR}/${CLI_NAME}" --version || true
}

main "$@"
