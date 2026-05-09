#!/usr/bin/env bash
# gen-runbook-quickref.sh — render docs/skycot/quickref.html from RUNBOOK.md.
#
# Hot-incident operators don't want to scroll a 1k-line runbook. This
# script extracts two named blocks from docs/skycot/RUNBOOK.md and
# renders them into a single self-contained HTML page (inline CSS, no
# external assets) at docs/skycot/quickref.html.
#
# Markers expected in RUNBOOK.md (must each sit on their own line):
#   <!-- quickref:commands:start --> ... <!-- quickref:commands:end -->
#   <!-- quickref:errors:start -->   ... <!-- quickref:errors:end -->
# Either marker pair missing → exit 2 with a clear message.
#
# Reproducibility: honors SOURCE_DATE_EPOCH if set; the footer
# timestamp is derived from it. Without it, the timestamp reflects
# the current UTC instant. Set SOURCE_DATE_EPOCH for byte-identical
# rebuilds (e.g. reproducible-build CI).
#
# Dependencies: bash, awk, sed, date, git. No Python, no Node.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/docs/skycot/RUNBOOK.md"
OUT="$ROOT/docs/skycot/quickref.html"

if [[ ! -f "$SRC" ]]; then
  echo "gen-runbook-quickref: source not found: $SRC" >&2
  exit 1
fi

require_marker() {
  if ! grep -qF "$1" "$SRC"; then
    echo "gen-runbook-quickref: missing marker '$1' in $SRC" >&2
    exit 2
  fi
}
require_marker "<!-- quickref:commands:start -->"
require_marker "<!-- quickref:commands:end -->"
require_marker "<!-- quickref:errors:start -->"
require_marker "<!-- quickref:errors:end -->"

# Extract lines strictly between (and excluding) the marker pair.
extract_section() {
  awk -v start="<!-- quickref:$1:start -->" \
      -v end="<!-- quickref:$1:end -->" '
    $0 == start { capture = 1; next }
    $0 == end   { exit }
    capture     { print }
  ' "$SRC"
}

CMDS_RAW=$(extract_section commands)
ERRS_RAW=$(extract_section errors)

# Strip markdown code fences and HTML-escape the body.
render_commands() {
  printf '%s\n' "$CMDS_RAW" \
    | awk '/^```/ { in_fence = !in_fence; next } in_fence { print }' \
    | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'
}

# Convert a markdown pipe-table to an HTML <table> body. Escaped
# pipes (`\|`) inside cells are stashed before split and restored
# after. Backticks become <code> spans in a final pass.
render_errors() {
  printf '%s\n' "$ERRS_RAW" \
    | sed \
        -e 's/&/\&amp;/g' \
        -e 's/</\&lt;/g' \
        -e 's/>/\&gt;/g' \
        -e 's/\\|/__QPIPE__/g' \
    | awk '
        BEGIN { row = 0 }
        /^\|/ {
          line = $0
          sub(/^\|[[:space:]]*/, "", line)
          sub(/[[:space:]]*\|[[:space:]]*$/, "", line)
          if (line ~ /^[-:|[:space:]]+$/) next
          row++
          n = split(line, cells, /[[:space:]]*\|[[:space:]]*/)
          if (row == 1) {
            printf "      <thead><tr>"
            for (i = 1; i <= n; i++) printf "<th>%s</th>", cells[i]
            printf "</tr></thead>\n      <tbody>\n"
          } else {
            printf "        <tr>"
            for (i = 1; i <= n; i++) printf "<td>%s</td>", cells[i]
            printf "</tr>\n"
          }
        }
        END { if (row > 0) printf "      </tbody>" }
      ' \
    | sed -e 's/__QPIPE__/|/g' -e "s/\`\\([^\`]*\\)\`/<code>\\1<\\/code>/g"
}

CMDS_HTML=$(render_commands)
ERRS_HTML=$(render_errors)

# Footer timestamp. SOURCE_DATE_EPOCH wins; macOS BSD date and GNU
# date have different flag spellings, so try both.
if [[ -n "${SOURCE_DATE_EPOCH:-}" ]]; then
  if GENERATED_AT=$(date -u -r "$SOURCE_DATE_EPOCH" +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null); then
    :
  else
    GENERATED_AT=$(date -u -d "@$SOURCE_DATE_EPOCH" +"%Y-%m-%dT%H:%M:%SZ")
  fi
else
  GENERATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
fi

SHA=$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)

cat > "$OUT" <<EOF
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>SKYCOT quick reference</title>
  <style>
    :root { color-scheme: light dark; }
    * { box-sizing: border-box; }
    body {
      margin: 0; padding: 1.5rem 2rem; max-width: 1100px;
      font: 14px/1.5 -apple-system, BlinkMacSystemFont,
            "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    }
    h1 { margin: 0 0 0.25rem; font-size: 1.4rem; }
    .subtitle { margin: 0 0 1.5rem; color: #666; font-size: 0.9rem; }
    section { margin-bottom: 1.5rem; }
    section h2 {
      margin: 0 0 0.5rem; font-size: 1rem;
      padding-bottom: 0.25rem; border-bottom: 1px solid #ccc;
    }
    .commands pre {
      margin: 0; padding: 0.75rem 1rem;
      background: #f5f5f5; border-radius: 4px;
      font-family: ui-monospace, SFMono-Regular,
                   "SF Mono", Consolas, monospace;
      font-size: 12.5px; line-height: 1.45; overflow-x: auto;
    }
    .errors table {
      width: 100%; border-collapse: collapse; font-size: 13px;
    }
    .errors th, .errors td {
      padding: 0.4rem 0.6rem; border: 1px solid #ddd;
      text-align: left; vertical-align: top;
    }
    .errors th { background: #f5f5f5; font-weight: 600; }
    code {
      font-family: ui-monospace, SFMono-Regular,
                   "SF Mono", Consolas, monospace;
      font-size: 0.92em;
      background: rgba(0, 0, 0, 0.06);
      padding: 1px 4px; border-radius: 3px;
    }
    pre code { background: none; padding: 0; }
    footer {
      margin-top: 2rem; padding-top: 0.75rem;
      border-top: 1px solid #eee;
      color: #888; font-size: 12px;
    }
    @media (prefers-color-scheme: dark) {
      body { background: #1a1a1a; color: #e6e6e6; }
      .subtitle, footer { color: #999; }
      section h2, footer { border-color: #333; }
      .commands pre, .errors th, code { background: #2a2a2a; }
      .errors th, .errors td { border-color: #333; }
    }
  </style>
</head>
<body>
  <h1>SKYCOT quick reference</h1>
  <p class="subtitle">
    Hot-incident card pulled from
    <code>docs/skycot/RUNBOOK.md</code>. Open the runbook for the
    full walkthrough.
  </p>

  <section class="commands">
    <h2>Top commands</h2>
    <pre>$CMDS_HTML</pre>
  </section>

  <section class="errors">
    <h2>Common error fixes</h2>
    <table>
$ERRS_HTML
    </table>
  </section>

  <footer>
    Generated $GENERATED_AT &middot; source $SHA &middot; regenerate with
    <code>scripts/gen-runbook-quickref.sh</code>
  </footer>
</body>
</html>
EOF

echo "wrote $OUT"
