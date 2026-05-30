#!/usr/bin/env python3
"""Render docs/build-plan/FEATURE_MATRIX.md → a styled, filterable HTML view.

The markdown is the source of truth; this generator keeps an at-a-glance,
color-coded HTML in sync. Re-run after editing the matrix:

    python3 scripts/render-feature-matrix.py

Output: docs/skycot/feature-matrix.html (self-contained, no deps).
"""
import re
import pathlib
import html as _html

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "docs/build-plan/FEATURE_MATRIX.md"
OUT = ROOT / "docs/skycot/feature-matrix.html"

STATUS = {
    "✅": ("live", "Live"),
    "🟡": ("partial", "Partial"),
    "🟧": ("scaffold", "Scaffold"),
    "🚧": ("inprogress", "In-progress"),
    "🗓️": ("planned", "Planned"),
    "💡": ("backlog", "Backlog"),
}

# Header labels whose COLUMN carries a status badge. Only cells in these
# columns are rendered as pills; every other cell renders as text even if it
# mentions a status emoji in its prose. This is the fix for the silent
# content-loss bug where a long evidence cell that happened to say
# "🟡 built + unit-verified" collapsed into a bare pill, discarding all its
# text — and where qualifiers like "✅ Live (macOS)" lost the qualifier.
STATUS_HEADERS = {"status", "backend", "demo surface", "badge"}

# Text label that follows the leading emoji in a status cell, to strip so it
# isn't printed twice (the pill already carries the canonical label).
_LABEL_RE = re.compile(r"^\s*(Live|Partial|Scaffold|In-progress|Planned|Backlog|Done)\b")


def status_columns(header):
    """Indices of columns that carry a status badge, by header label."""
    return {i for i, h in enumerate(header) if h.strip().lower() in STATUS_HEADERS}


def leading_status(cell: str):
    """If the cell *begins* with a status badge, return (class, label, rest).

    ``rest`` is any qualifier text after the badge ("✅ Live (macOS)" →
    rest=" (macOS)"; "🚧 In-progress / 🗓️ Planned" → rest=" / 🗓️ Planned").
    Returns ``None`` when the cell does not start with a known badge so the
    caller falls back to plain-text rendering.
    """
    stripped = cell.lstrip()
    for emoji, (cls, label) in STATUS.items():
        if stripped.startswith(emoji):
            rest = _LABEL_RE.sub("", stripped[len(emoji):])
            return cls, label, rest
    return None


def _rebase_link(href: str) -> str:
    """Rewrite a .md-relative link target so it works from the HTML's dir.

    Links in the source are relative to ``docs/build-plan/`` (where the .md
    lives); the HTML lives in ``docs/skycot/``, so a ``../skycot/x`` target
    becomes a plain ``x`` (already relative to the HTML). Absolute and
    fragment links pass through untouched.
    """
    if href.startswith("../skycot/"):
        return href[len("../skycot/"):]
    return href


def md_inline(s: str) -> str:
    # Convert markdown links FIRST (on the raw string) into placeholders so the
    # URL + label survive HTML-escaping intact, then restore as <a> tags.
    links = []

    def _stash(m):
        label, href = m.group(1), _rebase_link(m.group(2))
        links.append((_html.escape(label), _html.escape(href, quote=True)))
        return f"\x00LINK{len(links) - 1}\x00"

    s = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", _stash, s)
    s = _html.escape(s)
    s = re.sub(r"`([^`]+)`", r"<code>\1</code>", s)
    s = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", s)
    for i, (label, href) in enumerate(links):
        s = s.replace(f"\x00LINK{i}\x00", f'<a href="{href}">{label}</a>')
    return s


def render_status_cell(cell: str):
    """Return (html, status_class) for a cell in a status column.

    Renders a colored pill for the leading badge plus any trailing qualifier
    text as inline markdown. Falls back to plain text (and empty class) when
    the cell carries no recognizable badge.
    """
    hit = leading_status(cell)
    if not hit:
        return md_inline(cell), ""
    cls, label, rest = hit
    emoji = next(e for e, (c, _l) in STATUS.items() if c == cls)
    pill = f'<span class="pill {cls}">{emoji} {label}</span>'
    tail = md_inline(rest) if rest.strip() else ""
    return pill + tail, cls


def parse():
    lines = SRC.read_text(encoding="utf-8").splitlines()
    sections = []
    cur = None
    i = 0
    while i < len(lines):
        line = lines[i]
        m = re.match(r"^##\s+(.*)", line)
        if m:
            cur = {"title": m.group(1).strip(), "tables": [], "blurbs": []}
            sections.append(cur)
            i += 1
            continue
        # table block
        if cur is not None and line.strip().startswith("|") and i + 1 < len(lines) and re.match(r"^\|[\s:|-]+\|?\s*$", lines[i + 1].strip()):
            header = [c.strip() for c in line.strip().strip("|").split("|")]
            i += 2
            rows = []
            while i < len(lines) and lines[i].strip().startswith("|"):
                cells = [c.strip() for c in lines[i].strip().strip("|").split("|")]
                rows.append(cells)
                i += 1
            cur["tables"].append({"header": header, "rows": rows})
            continue
        i += 1
    return sections


def render():
    sections = parse()
    tallies = {cls: 0 for cls, _ in STATUS.values()}
    parts = []
    for sec in sections:
        if not sec["tables"]:
            continue
        sid = re.sub(r"[^a-z0-9]+", "-", sec["title"].lower()).strip("-")
        parts.append(f'<section><h2 id="{sid}">{md_inline(sec["title"])}</h2>')
        for tbl in sec["tables"]:
            scols = status_columns(tbl["header"])
            parts.append('<table><thead><tr>')
            for h in tbl["header"]:
                parts.append(f"<th>{md_inline(h)}</th>")
            parts.append("</tr></thead><tbody>")
            for row in tbl["rows"]:
                # The row's data-status (for filtering + tally) comes from the
                # FIRST status column only — never from prose cells.
                row_cls = ""
                for idx in sorted(scols):
                    if idx < len(row):
                        _h, cls = render_status_cell(row[idx])
                        if cls:
                            row_cls = cls
                            break
                if row_cls in tallies:
                    tallies[row_cls] += 1
                parts.append(f'<tr data-status="{row_cls}">')
                for i, c in enumerate(row):
                    if i in scols:
                        cell_html, _cls = render_status_cell(c)
                        parts.append(f"<td>{cell_html}</td>")
                    else:
                        parts.append(f"<td>{md_inline(c)}</td>")
                parts.append("</tr>")
            parts.append("</tbody></table>")
        parts.append("</section>")

    chips = "".join(
        f'<button class="chip {cls}" data-filter="{cls}">{label} <b>{tallies[cls]}</b></button>'
        for cls, label in [(c, l) for c, l in [(v[0], v[1]) for v in STATUS.values()]]
    )
    total = sum(tallies.values())
    body = "".join(parts)
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>SKYCOT — Feature Matrix & Roadmap</title>
<style>
:root{{--violet:#8B5CF6;--cyan:#0891B2;--ink:#1e1b4b;--line:rgba(15,23,42,.1);--bg:#fafaff}}
*{{box-sizing:border-box}}
body{{margin:0;font-family:Inter,system-ui,-apple-system,sans-serif;color:var(--ink);background:var(--bg);line-height:1.5}}
header{{background:linear-gradient(135deg,var(--violet),var(--cyan));color:#fff;padding:34px 28px 26px}}
header h1{{margin:0 0 4px;font-size:26px;letter-spacing:.3px}}
header p{{margin:0;opacity:.92;font-size:14px}}
.wrap{{max-width:1280px;margin:0 auto;padding:0 20px 80px}}
.bar{{position:sticky;top:0;z-index:5;background:rgba(250,250,255,.95);backdrop-filter:blur(8px);padding:14px 0;border-bottom:1px solid var(--line);display:flex;flex-wrap:wrap;gap:8px;align-items:center}}
.chip{{border:1px solid var(--line);background:#fff;border-radius:999px;padding:6px 13px;font:600 12px Inter,sans-serif;cursor:pointer;color:var(--ink);display:inline-flex;gap:6px;align-items:center}}
.chip b{{font-variant-numeric:tabular-nums}}
.chip.active{{outline:2px solid var(--violet);outline-offset:1px}}
.chip[data-filter=all]{{}}
h2{{margin:34px 0 12px;font-size:19px;border-left:4px solid var(--violet);padding-left:10px}}
table{{width:100%;border-collapse:collapse;font-size:13.5px;background:#fff;border:1px solid var(--line);border-radius:12px;overflow:hidden;margin-bottom:8px}}
th{{text-align:left;background:#f4f3ff;padding:9px 12px;font-size:11px;letter-spacing:.06em;text-transform:uppercase;color:#5b21b6;border-bottom:1px solid var(--line)}}
td{{padding:9px 12px;border-bottom:1px solid var(--line);vertical-align:top}}
tr:last-child td{{border-bottom:none}}
tr[data-status]:hover{{background:#faf9ff}}
code{{background:rgba(139,92,246,.09);color:#5b21b6;padding:1px 5px;border-radius:5px;font:12px ui-monospace,monospace}}
.pill{{display:inline-block;padding:3px 9px;border-radius:999px;font:700 11px Inter,sans-serif;white-space:nowrap}}
.pill.live{{background:rgba(34,197,94,.14);color:#166534}}
.pill.partial{{background:rgba(245,158,11,.16);color:#92400e}}
.pill.scaffold{{background:rgba(239,68,68,.13);color:#b91c1c}}
.pill.inprogress{{background:rgba(139,92,246,.16);color:#5b21b6}}
.pill.planned{{background:rgba(14,165,233,.14);color:#075985}}
.pill.backlog{{background:rgba(100,116,139,.15);color:#475569}}
.hide{{display:none!important}}
.foot{{margin-top:30px;font-size:12px;color:#64748b}}
</style></head>
<body>
<header><h1>SKYCOT — Feature Matrix &amp; Roadmap</h1>
<p>The honest, code-grounded source of truth · {total} capabilities · verified 2026-05-29 · generated from <code>docs/build-plan/FEATURE_MATRIX.md</code></p></header>
<div class="wrap">
<div class="bar"><button class="chip active" data-filter="all">All <b>{total}</b></button>{chips}</div>
{body}
<p class="foot">Generated by <code>scripts/render-feature-matrix.py</code> from the markdown source of truth. Re-run after editing the matrix. Status = ✅ Live · 🟡 Partial · 🟧 Scaffold (unwired) · 🚧 In-progress · 🗓️ Planned · 💡 Backlog.</p>
</div>
<script>
const chips=[...document.querySelectorAll('.chip')];
chips.forEach(c=>c.addEventListener('click',()=>{{
  chips.forEach(x=>x.classList.remove('active'));c.classList.add('active');
  const f=c.dataset.filter;
  document.querySelectorAll('tr[data-status]').forEach(r=>{{
    r.classList.toggle('hide', f!=='all' && r.dataset.status!==f);
  }});
  // hide sections with no visible rows
  document.querySelectorAll('section').forEach(s=>{{
    const vis=[...s.querySelectorAll('tr[data-status]')].some(r=>!r.classList.contains('hide'));
    s.classList.toggle('hide', f!=='all' && !vis);
  }});
}}));
</script>
</body></html>"""


OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text(render(), encoding="utf-8")
print(f"wrote {OUT}")
