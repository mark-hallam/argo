#!/usr/bin/env bash
# argo installer — one-liner install for macOS / Linux.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.sh | bash
#
# Honors:
#   ARGO_VERSION       — pin to a specific tag (default: latest release)
#   ARGO_INSTALL_DIR   — destination (default: ~/.local/bin)
#
# Windows users: download argo-x86_64-pc-windows-msvc.zip from the
# Releases page directly. (No PowerShell version of this script
# yet — open an issue if you want one.)

set -euo pipefail

REPO="mark-hallam/Argo-Agent"
INSTALL_DIR="${ARGO_INSTALL_DIR:-$HOME/.local/bin}"

# ---- detect platform -------------------------------------------------

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"

case "$os" in
  darwin)
    case "$arch" in
      arm64|aarch64)  target="aarch64-apple-darwin" ;;
      x86_64)         target="x86_64-apple-darwin" ;;
      *)              target="" ;;
    esac
    ;;
  linux)
    case "$arch" in
      x86_64)         target="x86_64-unknown-linux-gnu" ;;
      aarch64|arm64)  target="aarch64-unknown-linux-gnu" ;;
      *)              target="" ;;
    esac
    ;;
  *)  target="" ;;
esac

if [ -z "$target" ]; then
  echo "argo: no pre-built binary for $os/$arch."
  echo "      build from source: https://github.com/$REPO#build-from-source"
  exit 1
fi

# ---- pick a release tag ---------------------------------------------

if [ -n "${ARGO_VERSION:-}" ]; then
  tag="$ARGO_VERSION"
else
  tag="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
          | grep -m1 '"tag_name"' \
          | cut -d'"' -f4 || true)"
fi

if [ -z "$tag" ]; then
  echo "argo: couldn't find a release. Either:"
  echo "  - no release has been cut yet (ask the maintainer to run \`git tag v0.1.0 && git push --tags\`)"
  echo "  - the GitHub API is unreachable (check your network)"
  echo "  - or pin a specific tag: ARGO_VERSION=v0.1.0 ./install.sh"
  exit 1
fi

asset="argo-$target.tar.gz"
url="https://github.com/$REPO/releases/download/$tag/$asset"

echo "argo: installing $tag for $target"
echo "      from $url"

# ---- download + extract ---------------------------------------------

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

if ! curl -fsSL "$url" -o "$tmp/$asset"; then
  echo "argo: download failed. Available assets:"
  echo "      https://github.com/$REPO/releases/tag/$tag"
  exit 1
fi

tar xzf "$tmp/$asset" -C "$tmp"

if [ ! -x "$tmp/argo" ]; then
  echo "argo: tarball did not contain an argo binary"
  exit 1
fi

# ---- install --------------------------------------------------------

mkdir -p "$INSTALL_DIR"
mv "$tmp/argo" "$INSTALL_DIR/argo"
chmod +x "$INSTALL_DIR/argo"

# ---- PATH check -----------------------------------------------------

case ":${PATH:-}:" in
  *":$INSTALL_DIR:"*)
    PATH_OK=1
    ;;
  *)
    PATH_OK=0
    ;;
esac

echo ""
echo "argo $tag installed to $INSTALL_DIR/argo"
echo ""

if [ "$PATH_OK" -eq 0 ]; then
  echo "Note: $INSTALL_DIR is not on your PATH."
  echo "      Add this to your shell rc (~/.bashrc / ~/.zshrc):"
  echo ""
  echo "      export PATH=\"$INSTALL_DIR:\$PATH\""
  echo ""
fi

echo "Next: run \`argo setup\` to configure your workspace."
