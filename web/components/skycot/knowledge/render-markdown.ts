/**
 * Tiny, dependency-free markdown renderer for the wiki Preview tab.
 *
 * Handles the subset every workplace doc needs:
 * - ATX headings (`#`, `##`, `###`)
 * - Inline `**bold**`, `*italic*`, ``code``
 * - Bullet lists (`- item`) + numbered lists (`1. item`)
 * - Fenced code blocks (``` … ```)
 * - Paragraphs (blank line separated)
 *
 * NOT full CommonMark — no nested lists, blockquotes, tables, footnotes,
 * autolinks, or HTML. The gotcha note on the brief notes that vendoring
 * `react-markdown` would touch `web/package.json` + `pnpm-lock.yaml`
 * and add ~25KB to the bundle; for a workplace handbook the surface
 * needed is small enough that an inline renderer beats a dep.
 *
 * Safety: every text segment is HTML-escaped before composition; only
 * the deterministic tag set above is emitted. The return value is a
 * string of safe HTML the caller drops via `dangerouslySetInnerHTML`.
 * Tests cover the round-trip and verify no `<script>` / `<iframe>` /
 * `onerror=` slips through.
 */

const ESC_MAP: Record<string, string> = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

export function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (ch) => ESC_MAP[ch] ?? ch);
}

function renderInline(line: string): string {
  let out = escapeHtml(line);
  out = out.replace(/`([^`]+)`/g, (_m, code) => `<code>${code}</code>`);
  out = out.replace(/\*\*([^*]+)\*\*/g, (_m, b) => `<strong>${b}</strong>`);
  out = out.replace(/\*([^*]+)\*/g, (_m, i) => `<em>${i}</em>`);
  return out;
}

interface Block {
  kind: "h1" | "h2" | "h3" | "p" | "ul" | "ol" | "code";
  lines: string[];
}

function tokenize(src: string): Block[] {
  const blocks: Block[] = [];
  const lines = src.replace(/\r\n/g, "\n").split("\n");
  let i = 0;
  while (i < lines.length) {
    const raw = lines[i];
    if (raw.startsWith("```")) {
      const code: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) {
        code.push(lines[i]);
        i++;
      }
      if (i < lines.length) i++;
      blocks.push({ kind: "code", lines: code });
      continue;
    }
    if (raw.trim() === "") {
      i++;
      continue;
    }
    if (raw.startsWith("### ")) {
      blocks.push({ kind: "h3", lines: [raw.slice(4)] });
      i++;
      continue;
    }
    if (raw.startsWith("## ")) {
      blocks.push({ kind: "h2", lines: [raw.slice(3)] });
      i++;
      continue;
    }
    if (raw.startsWith("# ")) {
      blocks.push({ kind: "h1", lines: [raw.slice(2)] });
      i++;
      continue;
    }
    if (/^[-*] /.test(raw)) {
      const items: string[] = [];
      while (i < lines.length && /^[-*] /.test(lines[i])) {
        items.push(lines[i].slice(2));
        i++;
      }
      blocks.push({ kind: "ul", lines: items });
      continue;
    }
    if (/^\d+\.\s/.test(raw)) {
      const items: string[] = [];
      while (i < lines.length && /^\d+\.\s/.test(lines[i])) {
        items.push(lines[i].replace(/^\d+\.\s/, ""));
        i++;
      }
      blocks.push({ kind: "ol", lines: items });
      continue;
    }
    const para: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !lines[i].startsWith("#") &&
      !lines[i].startsWith("```") &&
      !/^[-*] /.test(lines[i]) &&
      !/^\d+\.\s/.test(lines[i])
    ) {
      para.push(lines[i]);
      i++;
    }
    blocks.push({ kind: "p", lines: para });
  }
  return blocks;
}

export function renderMarkdown(src: string): string {
  const blocks = tokenize(src);
  const html: string[] = [];
  for (const block of blocks) {
    switch (block.kind) {
      case "h1":
        html.push(`<h1>${renderInline(block.lines[0])}</h1>`);
        break;
      case "h2":
        html.push(`<h2>${renderInline(block.lines[0])}</h2>`);
        break;
      case "h3":
        html.push(`<h3>${renderInline(block.lines[0])}</h3>`);
        break;
      case "p":
        html.push(`<p>${block.lines.map(renderInline).join("<br/>")}</p>`);
        break;
      case "ul":
        html.push(`<ul>${block.lines.map((l) => `<li>${renderInline(l)}</li>`).join("")}</ul>`);
        break;
      case "ol":
        html.push(`<ol>${block.lines.map((l) => `<li>${renderInline(l)}</li>`).join("")}</ol>`);
        break;
      case "code":
        html.push(`<pre><code>${escapeHtml(block.lines.join("\n"))}</code></pre>`);
        break;
    }
  }
  return html.join("\n");
}
