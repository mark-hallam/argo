#!/usr/bin/env bash
# Render docs/skycot-system-overview.html to a launch-quality PDF
# at docs/skycot-system-overview.pdf using Chrome headless.
#
# Inputs:
#   docs/skycot-system-overview.html   — structure
#   docs/skycot-system-overview.css    — print stylesheet
# Output:
#   docs/skycot-system-overview.pdf
#
# Edit the .md source for content, mirror copy changes into the
# .html, then re-run this script.
#
# Requires: Google Chrome installed at the standard macOS path,
# OR `chromium` / `google-chrome` on PATH.

set -euo pipefail
trap 'echo; echo "FAILED at line $LINENO" >&2' ERR

DOCS_DIR="$(cd "$(dirname "$0")" && pwd)"
INPUT="file://${DOCS_DIR}/skycot-system-overview.html"
OUTPUT="${DOCS_DIR}/skycot-system-overview.pdf"

# Resolve a Chrome binary. Order: explicit env, macOS app bundle,
# common Linux locations.
CHROME="${CHROME:-}"
if [ -z "$CHROME" ]; then
  if [ -x "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" ]; then
    CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
  elif command -v google-chrome >/dev/null 2>&1; then
    CHROME="$(command -v google-chrome)"
  elif command -v chromium >/dev/null 2>&1; then
    CHROME="$(command -v chromium)"
  elif command -v chromium-browser >/dev/null 2>&1; then
    CHROME="$(command -v chromium-browser)"
  else
    echo "FAIL: Google Chrome / Chromium not found." >&2
    echo "  Set CHROME=/path/to/chrome and re-run." >&2
    exit 1
  fi
fi

echo "==> using browser: $CHROME"
echo "==> rendering    : $INPUT"
echo "==> output       : $OUTPUT"

# --no-pdf-header-footer drops the default URL/page footer.
# --virtual-time-budget gives any deferred CSS layout time to settle.
"$CHROME" \
  --headless=new \
  --disable-gpu \
  --no-pdf-header-footer \
  --virtual-time-budget=4000 \
  --print-to-pdf="$OUTPUT" \
  --print-to-pdf-no-header \
  "$INPUT" \
  >/dev/null 2>&1 \
    || { echo "FAIL: chrome render exited non-zero" >&2; exit 1; }

if [ ! -s "$OUTPUT" ]; then
  echo "FAIL: PDF was not produced (or is empty)" >&2; exit 1
fi

SIZE=$(wc -c < "$OUTPUT" | awk '{print $1}')
echo "==> wrote $OUTPUT ($((SIZE/1024)) KB)"
