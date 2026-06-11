#!/bin/sh
# dsi installer — downloads the dsi binary from GitHub Releases and configures PATH.
#
#   curl -fsSL https://raw.githubusercontent.com/curllog/dsi/main/install.sh | sh
#
# It always installs the latest release to ~/.local/bin/dsi.
# This script is POSIX sh — no bashisms.

set -eu

# ─── Config ───────────────────────────────────────────────────────────────
DSI_REPO="curllog/dsi"
DSI_BIN_DIR="$HOME/.local/bin"
DOTNET_ROOT="$HOME/.dotnet"

MARKER_BEGIN="# >>> dsi >>>"
MARKER_END="# <<< dsi <<<"

# Core 6 targets actually published by the release workflow.
SUPPORTED_RIDS="linux-x64 linux-arm64 linux-musl-x64 linux-musl-arm64 osx-x64 osx-arm64"

# ─── Output helpers ───────────────────────────────────────────────────────
if [ -t 1 ]; then
  C_BLUE="$(printf '\033[34m')"; C_YELLOW="$(printf '\033[33m')"
  C_RED="$(printf '\033[31m')"; C_GREEN="$(printf '\033[32m')"; C_RESET="$(printf '\033[0m')"
else
  C_BLUE=""; C_YELLOW=""; C_RED=""; C_GREEN=""; C_RESET=""
fi

info() { printf '%s==>%s %s\n' "$C_BLUE" "$C_RESET" "$1"; }
warn() { printf '%swarning:%s %s\n' "$C_YELLOW" "$C_RESET" "$1" >&2; }
err()  { printf '%serror:%s %s\n' "$C_RED" "$C_RESET" "$1" >&2; exit 1; }

# ─── Tooling detection ────────────────────────────────────────────────────
if command -v curl >/dev/null 2>&1; then
  DL="curl"
elif command -v wget >/dev/null 2>&1; then
  DL="wget"
else
  err "need either curl or wget installed"
fi

command -v tar >/dev/null 2>&1 || err "need tar installed"

if command -v sha256sum >/dev/null 2>&1; then
  SHA="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
  SHA="shasum -a 256"
else
  err "need sha256sum or shasum installed"
fi

# download <url> <dest-file>
download() {
  if [ "$DL" = "curl" ]; then
    curl -fsSL "$1" -o "$2"
  else
    wget -qO "$2" "$1"
  fi
}

# fetch <url> -> stdout
fetch() {
  if [ "$DL" = "curl" ]; then
    curl -fsSL "$1"
  else
    wget -qO- "$1"
  fi
}

# ─── RID detection (mirrors src/platform.rs::rid) ─────────────────────────
detect_rid() {
  os_raw="$(uname -s)"
  arch_raw="$(uname -m)"

  case "$os_raw" in
    Linux) os="linux" ;;
    Darwin) os="osx" ;;
    *) err "unsupported OS: $os_raw (dsi supports Linux, macOS, and WSL)" ;;
  esac

  case "$arch_raw" in
    x86_64|amd64) arch="x64" ;;
    aarch64|arm64) arch="arm64" ;;
    armv7l|armv6l|armhf|arm) arch="arm" ;;
    i686|i386|x86) arch="x86" ;;
    *) err "unsupported architecture: $arch_raw" ;;
  esac

  # musl vs glibc — only relevant on Linux. Mirror detect_libc().
  if [ "$os" = "linux" ]; then
    if ldd --version 2>&1 | grep -qi musl; then
      os="linux-musl"
    elif [ -e /lib/ld-musl-x86_64.so.1 ] || [ -e /lib/ld-musl-aarch64.so.1 ] || [ -e /lib/ld-musl-armhf.so.1 ]; then
      os="linux-musl"
    fi
  fi

  printf '%s-%s' "$os" "$arch"
}

# ─── Latest release tag ───────────────────────────────────────────────────
resolve_tag() {
  # Parse "tag_name": "vX.Y.Z" from the GitHub API without jq.
  tag="$(fetch "https://api.github.com/repos/$DSI_REPO/releases/latest" \
    | grep '"tag_name"' | head -n1 | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')"
  [ -n "$tag" ] || err "could not determine latest release of $DSI_REPO"
  printf '%s' "$tag"
}

# ─── PATH / env configuration ─────────────────────────────────────────────
# apply_block <profile-file> <style>
# style: bash | zsh | fish | powershell | nushell
# Idempotent: removes any prior dsi block, then appends a fresh one.
# Completion lines are guarded so they silently no-op if the installed dsi
# predates the `completions` subcommand.
apply_block() {
  file="$1"; style="$2"
  dir="$(dirname "$file")"
  [ -d "$dir" ] || mkdir -p "$dir"
  [ -f "$file" ] || : > "$file"

  # Strip an existing dsi block if present.
  if grep -qF "$MARKER_BEGIN" "$file" 2>/dev/null; then
    tmp="$(mktemp)"
    awk -v b="$MARKER_BEGIN" -v e="$MARKER_END" '
      $0==b {skip=1; next}
      $0==e {skip=0; next}
      skip!=1 {print}
    ' "$file" > "$tmp"
    cat "$tmp" > "$file"
    rm -f "$tmp"
  fi

  {
    printf '%s\n' "$MARKER_BEGIN"
    case "$style" in
      bash)
        printf 'export DOTNET_ROOT="%s"\n' "$DOTNET_ROOT"
        printf 'export PATH="%s:%s/tools:%s:$PATH"\n' "$DOTNET_ROOT" "$DOTNET_ROOT" "$DSI_BIN_DIR"
        printf 'command -v dsi >/dev/null 2>&1 && eval "$(dsi completions bash 2>/dev/null)"\n'
        ;;
      zsh)
        printf 'export DOTNET_ROOT="%s"\n' "$DOTNET_ROOT"
        printf 'export PATH="%s:%s/tools:%s:$PATH"\n' "$DOTNET_ROOT" "$DOTNET_ROOT" "$DSI_BIN_DIR"
        # zsh completions need compinit (compdef) to be loaded first.
        printf 'command -v dsi >/dev/null 2>&1 && command -v compdef >/dev/null 2>&1 && eval "$(dsi completions zsh 2>/dev/null)"\n'
        ;;
      fish)
        printf 'set -gx DOTNET_ROOT "%s"\n' "$DOTNET_ROOT"
        printf 'fish_add_path "%s" "%s/tools" "%s"\n' "$DOTNET_ROOT" "$DOTNET_ROOT" "$DSI_BIN_DIR"
        printf 'command -q dsi; and dsi completions fish 2>/dev/null | source\n'
        ;;
      powershell)
        printf '$env:DOTNET_ROOT = "%s"\n' "$DOTNET_ROOT"
        printf '$env:PATH = "%s:%s/tools:%s:$env:PATH"\n' "$DOTNET_ROOT" "$DOTNET_ROOT" "$DSI_BIN_DIR"
        printf 'if (Get-Command dsi -ErrorAction SilentlyContinue) { dsi completions powershell 2>$null | Out-String | Invoke-Expression }\n'
        ;;
      nushell)
        printf '$env.DOTNET_ROOT = "%s"\n' "$DOTNET_ROOT"
        printf '$env.PATH = ($env.PATH | prepend ["%s" "%s/tools" "%s"])\n' "$DOTNET_ROOT" "$DOTNET_ROOT" "$DSI_BIN_DIR"
        ;;
    esac
    printf '%s\n' "$MARKER_END"
  } >> "$file"

  info "configured $file"
}

configure_path() {
  configured=0

  if command -v bash >/dev/null 2>&1 || [ -f "$HOME/.bashrc" ]; then
    apply_block "$HOME/.bashrc" bash; configured=1
  fi
  if command -v zsh >/dev/null 2>&1 || [ -f "${ZDOTDIR:-$HOME}/.zshrc" ]; then
    apply_block "${ZDOTDIR:-$HOME}/.zshrc" zsh; configured=1
  fi
  if command -v fish >/dev/null 2>&1 || [ -f "$HOME/.config/fish/config.fish" ]; then
    apply_block "$HOME/.config/fish/config.fish" fish; configured=1
  fi
  if command -v pwsh >/dev/null 2>&1 || [ -f "$HOME/.config/powershell/Microsoft.PowerShell_profile.ps1" ]; then
    apply_block "$HOME/.config/powershell/Microsoft.PowerShell_profile.ps1" powershell; configured=1
  fi
  if command -v nu >/dev/null 2>&1 || [ -f "$HOME/.config/nushell/config.nu" ]; then
    apply_block "$HOME/.config/nushell/config.nu" nushell; configured=1
  fi

  [ "$configured" -eq 1 ] || warn "no known shell profiles found; add $DSI_BIN_DIR and $DOTNET_ROOT to your PATH manually"
}

# Print the reload command for the user's current shell.
# Detects the shell from $SHELL and emits the matching source/reload line.
reload_hint() {
  shell_name="$(basename "${SHELL:-}")"

  case "$shell_name" in
    bash)
      printf 'source ~/.bashrc' ;;
    zsh)
      if [ -n "${ZDOTDIR:-}" ]; then
        printf 'source "%s/.zshrc"' "$ZDOTDIR"
      else
        printf 'source ~/.zshrc'
      fi ;;
    fish)
      printf 'source ~/.config/fish/config.fish' ;;
    pwsh|powershell)
      printf '. $PROFILE' ;;
    nu)
      printf 'source ~/.config/nushell/config.nu' ;;
    *)
      # Unknown shell — fall back to a generic instruction.
      printf '' ;;
  esac
}

# Warn if a system dotnet would shadow the user-level one.
warn_system_dotnet() {
  if command -v dotnet >/dev/null 2>&1; then
    existing="$(command -v dotnet)"
    case "$existing" in
      "$DOTNET_ROOT"/*) : ;;
      *) warn "a system dotnet was found at $existing; the profile prepends $DOTNET_ROOT so user-managed SDKs take precedence after restart" ;;
    esac
  fi
}

# ─── Main ─────────────────────────────────────────────────────────────────
main() {
  rid="$(detect_rid)"

  case " $SUPPORTED_RIDS " in
    *" $rid "*) : ;;
    *) err "no prebuilt binary for '$rid'. Supported: $SUPPORTED_RIDS" ;;
  esac

  tag="$(resolve_tag)"
  info "installing dsi $tag for $rid"

  asset="dsi-$rid.tar.gz"
  base="https://github.com/$DSI_REPO/releases/download/$tag"

  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT INT TERM

  info "downloading $asset"
  download "$base/$asset" "$tmp/$asset" || err "failed to download $base/$asset"
  download "$base/$asset.sha256" "$tmp/$asset.sha256" || err "failed to download checksum"

  info "verifying checksum"
  expected="$(awk '{print $1; exit}' "$tmp/$asset.sha256")"
  actual="$(cd "$tmp" && $SHA "$asset" | awk '{print $1}')"
  [ -n "$expected" ] || err "empty checksum file"
  if [ "$expected" != "$actual" ]; then
    err "checksum mismatch (expected $expected, got $actual) — aborting"
  fi

  info "extracting"
  tar -xzf "$tmp/$asset" -C "$tmp"
  [ -f "$tmp/dsi" ] || err "archive did not contain a dsi binary"
  chmod +x "$tmp/dsi"

  mkdir -p "$DSI_BIN_DIR"
  mv -f "$tmp/dsi" "$DSI_BIN_DIR/dsi"
  info "installed dsi to $DSI_BIN_DIR/dsi"

  configure_path
  warn_system_dotnet

  printf '\n%s✓ dsi %s installed%s\n' "$C_GREEN" "$tag" "$C_RESET"
  "$DSI_BIN_DIR/dsi" --version 2>/dev/null || true

  reload="$(reload_hint)"
  if [ -n "$reload" ]; then
    printf '\nReload your shell so dsi is on PATH:\n  %s\n\nThen:\n  dsi install --lts\n' "$reload"
  else
    printf '\nRestart your shell (or source your profile) so dsi is on PATH, then:\n  dsi install --lts\n'
  fi
}

main
