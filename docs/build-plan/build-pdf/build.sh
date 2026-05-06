#!/usr/bin/env bash
# Reproducible build of docs/build-plan/PROTOCOL_ARCHITECTURE.pdf.
#
# Pipeline:
#   1. Render Mermaid diagrams (.mmd → .svg) via mmdc.
#   2. Render PROTOCOL.md to HTML via pandoc.
#   3. Render HTML to PDF via headless Google Chrome.
#
# Re-run: ./build.sh from this directory.
# Outputs: ../PROTOCOL_ARCHITECTURE.pdf (committed).
#
# Inputs (all in this directory):
#   diagrams/01-architecture.mmd      → architecture overview
#   diagrams/02-brief-lifecycle.mmd   → brief state machine
#   diagrams/03-conductor-tick.mmd    → conductor 9-step loop
#   diagrams/04-worker-tick.mmd       → worker 15-step loop
#   diagrams/05-race.mmd              → first-push-wins race sequence
#   diagrams/config.json              → mermaid theme config
#   diagrams/puppeteer-config.json    → chrome path for mmdc
#   style.css                         → print stylesheet (A4)
#   header.html                       → HTML <head> (metadata + CSS link)
#   build.sh                          → this script
#
# Dependencies (one-time, outside this script):
#   pandoc            (brew install pandoc)
#   Google Chrome     (/Applications/Google Chrome.app)
#   mermaid-cli       (npm install in this directory — package.json pinned)
#
set -euo pipefail

cd "$(dirname "$0")"

PROTOCOL_MD="../PROTOCOL.md"
OUT_PDF="../PROTOCOL_ARCHITECTURE.pdf"
WORK="$(mktemp -d -t skycot-pdf-XXXXXX)"
trap 'rm -rf "$WORK"' EXIT

CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
MMDC="./node_modules/.bin/mmdc"

if [[ ! -x "$CHROME" ]]; then
  echo "error: Chrome not found at $CHROME" >&2; exit 1
fi
if [[ ! -x "$MMDC" ]]; then
  echo "error: mermaid-cli not installed; run: npm install" >&2; exit 1
fi
if ! command -v pandoc >/dev/null; then
  echo "error: pandoc not on PATH; brew install pandoc" >&2; exit 1
fi

echo "[1/3] Rendering Mermaid diagrams..."
for src in diagrams/*.mmd; do
  out="${src%.mmd}.svg"
  if [[ ! -f "$out" || "$src" -nt "$out" ]]; then
    echo "  $src → $out"
    "$MMDC" -i "$src" -o "$out" \
      -c diagrams/config.json \
      -p diagrams/puppeteer-config.json \
      -b white >/dev/null
  else
    echo "  $out up to date"
  fi
done

echo "[2/3] Rendering PROTOCOL.md → HTML..."
# Stage the markdown into the work dir so embedded image paths resolve relative
# to the build-pdf directory (where the SVGs live).
cp "$PROTOCOL_MD" "$WORK/PROTOCOL.md"

pandoc \
  --from=gfm \
  --to=html5 \
  --standalone \
  --metadata title="" \
  --include-in-header=header.html \
  --include-before-body=cover.html \
  "$WORK/PROTOCOL.md" \
  -o "$WORK/PROTOCOL.html"

# Inline the CSS so Chrome can find it from --headless.
# (Chrome --print-to-pdf doesn't always honor relative <link> URLs robustly.)
python3 inline_css.py "$WORK/PROTOCOL.html" style.css "$WORK/PROTOCOL.inlined.html"

# Inline SVGs from this directory's diagrams/ — pandoc emits <img src="diagrams/...svg">,
# we substitute the file content so the headless Chrome run is self-contained.
python3 inline_svgs.py "$WORK/PROTOCOL.inlined.html" diagrams "$WORK/PROTOCOL.final.html"

echo "[3/3] Rendering HTML → PDF (headless Chrome)..."
"$CHROME" \
  --headless=new \
  --no-sandbox \
  --disable-gpu \
  --no-pdf-header-footer \
  --print-to-pdf="$OUT_PDF" \
  "file://$WORK/PROTOCOL.final.html" >/dev/null 2>&1

# Show output stats.
size=$(stat -f%z "$OUT_PDF")
size_h=$(printf "%d" "$size" | awk '{ printf("%.0f KB", $1/1024) }')
echo
echo "Built: $OUT_PDF ($size_h)"
echo "  Verify: open $OUT_PDF"
