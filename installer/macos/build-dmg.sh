#!/usr/bin/env bash
# build-dmg.sh — package a release-mode `argo` binary into a signed
# (or ad-hoc signed if no Apple Developer cert is available) DMG.
#
# Inputs (env vars):
#   ARGO_BINARY        path to the prebuilt release binary (required)
#   ARGO_VERSION       version string to embed in the DMG name (required)
#   ARGO_SIGN_IDENTITY codesign identity. Default: "-" (ad-hoc).
#   ARGO_OUTPUT_DIR    where to drop the .dmg. Default: ./dist
#
# Usage:
#   ARGO_BINARY=target/release/argo ARGO_VERSION=0.1.0 \
#     installer/macos/build-dmg.sh
#
# Exit codes:
#   0 — DMG built
#   1 — missing input
#   2 — codesign failure
#   3 — hdiutil failure
set -euo pipefail

ARGO_BINARY="${ARGO_BINARY:-}"
ARGO_VERSION="${ARGO_VERSION:-}"
ARGO_SIGN_IDENTITY="${ARGO_SIGN_IDENTITY:--}"
ARGO_OUTPUT_DIR="${ARGO_OUTPUT_DIR:-./dist}"

if [[ -z "$ARGO_BINARY" || ! -f "$ARGO_BINARY" ]]; then
    echo "build-dmg.sh: ARGO_BINARY not set or file missing: $ARGO_BINARY" >&2
    exit 1
fi
if [[ -z "$ARGO_VERSION" ]]; then
    echo "build-dmg.sh: ARGO_VERSION must be set" >&2
    exit 1
fi

mkdir -p "$ARGO_OUTPUT_DIR"
SCRATCH="$(mktemp -d)"
STAGING="$SCRATCH/argo-${ARGO_VERSION}"
mkdir -p "$STAGING"

cp "$ARGO_BINARY" "$STAGING/argo"
chmod +x "$STAGING/argo"

INSTALLER_DIR="$(cd "$(dirname "$0")" && pwd)"
cp "$INSTALLER_DIR/post-install.sh" "$STAGING/post-install.sh"
cp "$INSTALLER_DIR/uninstall.sh" "$STAGING/uninstall.sh"
chmod +x "$STAGING/post-install.sh" "$STAGING/uninstall.sh"

cat > "$STAGING/README.txt" <<EOF
Argo ${ARGO_VERSION} — macOS distribution

Installation
  1. Copy the \`argo\` binary into \`/usr/local/bin\` (or any directory on
     your PATH):
         sudo cp argo /usr/local/bin/argo
  2. Run the post-install hint:
         ./post-install.sh
  3. Optional — install the launchd agent so the gateway starts on
     login:
         argo daemon install

Uninstallation
  Run \`./uninstall.sh\`.
EOF

if ! codesign --force --sign "$ARGO_SIGN_IDENTITY" "$STAGING/argo"; then
    echo "build-dmg.sh: codesign failed (identity=$ARGO_SIGN_IDENTITY)" >&2
    exit 2
fi

DMG="$ARGO_OUTPUT_DIR/argo-${ARGO_VERSION}-macos.dmg"
rm -f "$DMG"
if ! hdiutil create -volname "Argo ${ARGO_VERSION}" \
        -srcfolder "$STAGING" \
        -ov -format UDZO \
        "$DMG"; then
    echo "build-dmg.sh: hdiutil create failed" >&2
    exit 3
fi

if ! codesign --force --sign "$ARGO_SIGN_IDENTITY" "$DMG"; then
    echo "build-dmg.sh: codesign of DMG failed (identity=$ARGO_SIGN_IDENTITY)" >&2
    exit 2
fi

echo "$DMG"
