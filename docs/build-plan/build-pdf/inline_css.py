#!/usr/bin/env python3
"""Inline a CSS file into an HTML document.

Replaces the <style></style> block produced by pandoc + header.html with the
contents of the supplied CSS file. Lets us avoid fighting Chrome's
relative-path handling under --headless.

Usage: inline_css.py <input.html> <stylesheet.css> <output.html>
"""
import sys
import re

if len(sys.argv) != 4:
    sys.exit("usage: inline_css.py <input.html> <stylesheet.css> <output.html>")

in_html, css_path, out_html = sys.argv[1:]

with open(css_path, encoding="utf-8") as f:
    css = f.read()

with open(in_html, encoding="utf-8") as f:
    html = f.read()

# Replace the FIRST <style>...</style> in <head> with the actual CSS content.
# pandoc inlines header.html into <head>, so the placeholder <style> from
# header.html is the one we want to overwrite.
new_block = f"<style>\n{css}\n</style>"
html2, n = re.subn(
    r"<style>.*?</style>",
    lambda m: new_block,
    html,
    count=1,
    flags=re.DOTALL,
)
if n == 0:
    # No <style> block; inject one before </head>.
    html2 = html.replace("</head>", f"{new_block}\n</head>", 1)

with open(out_html, "w", encoding="utf-8") as f:
    f.write(html2)

print(f"  inlined {len(css)} bytes of CSS → {out_html}")
