#!/usr/bin/env python3
"""Inline SVG files referenced by <img src="diagrams/*.svg">.

Walks the HTML, finds <img> tags pointing at the diagrams directory, and
replaces them with the SVG file's literal contents wrapped in a <figure>.

This makes the HTML self-contained so headless Chrome can render it from
a tmp directory without resolving relative paths.

Usage: inline_svgs.py <input.html> <diagrams_dir> <output.html>
"""
import sys
import re
import os
from pathlib import Path

if len(sys.argv) != 4:
    sys.exit("usage: inline_svgs.py <input.html> <diagrams_dir> <output.html>")

in_html, dia_dir, out_html = sys.argv[1:]
dia_dir = Path(dia_dir).resolve()

with open(in_html, encoding="utf-8") as f:
    html = f.read()

count = 0
def replace(match):
    global count
    src = match.group(1)
    alt = match.group(2) or ""
    if "diagrams/" not in src:
        return match.group(0)
    fname = os.path.basename(src)
    svg_path = dia_dir / fname
    if not svg_path.exists():
        # Try alternate names without subdir prefix.
        candidates = list(dia_dir.glob(fname))
        if candidates:
            svg_path = candidates[0]
        else:
            print(f"  WARNING: SVG not found: {svg_path}", file=sys.stderr)
            return match.group(0)
    svg = svg_path.read_text(encoding="utf-8")
    # Strip any XML declaration for clean inlining.
    svg = re.sub(r"^<\?xml[^?]+\?>\s*", "", svg)
    count += 1
    figcaption = f'<figcaption>{alt}</figcaption>' if alt else ''
    return f'<figure class="diagram">{svg}{figcaption}</figure>'

# Match <img src="..." alt="..." /> in either order.
pattern = re.compile(
    r'<img\s+[^>]*?src="([^"]+)"(?:\s+alt="([^"]*)")?[^>]*/?>',
    re.IGNORECASE,
)
html2 = pattern.sub(replace, html)

# Also handle <img alt="..." src="..."> ordering.
pattern2 = re.compile(
    r'<img\s+alt="([^"]*)"\s+src="([^"]+)"[^>]*/?>',
    re.IGNORECASE,
)
def replace2(match):
    global count
    alt = match.group(1)
    src = match.group(2)
    if "diagrams/" not in src:
        return match.group(0)
    fname = os.path.basename(src)
    svg_path = dia_dir / fname
    if not svg_path.exists():
        return match.group(0)
    svg = svg_path.read_text(encoding="utf-8")
    svg = re.sub(r"^<\?xml[^?]+\?>\s*", "", svg)
    count += 1
    figcaption = f'<figcaption>{alt}</figcaption>' if alt else ''
    return f'<figure class="diagram">{svg}{figcaption}</figure>'
html2 = pattern2.sub(replace2, html2)

with open(out_html, "w", encoding="utf-8") as f:
    f.write(html2)

print(f"  inlined {count} SVG diagram(s) → {out_html}")
