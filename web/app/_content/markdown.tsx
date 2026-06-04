// Minimal, dependency-free markdown renderer for ported public content
// (the blog + legal/DPA pages that came over from the old `marketing/`
// app during the single-app consolidation). Hand-rolled on purpose:
// `web/` has no markdown/MDX dependency and we did not want to add one
// just to render three prose essays and a legal template.
//
// Supported subset (everything the ported content actually uses):
// h1-h3, paragraphs, bold, italic, inline code, links, fenced code
// blocks, unordered + ordered lists. MDX-style comments (`{/* ... */}`)
// are stripped so the old `.mdx` bodies render cleanly as `.md`.

import type { ReactElement, ReactNode } from "react";

type Block =
  | { kind: "h1" | "h2" | "h3" | "p"; text: string }
  | { kind: "ul" | "ol"; items: string[] }
  | { kind: "code"; text: string };

function renderInline(text: string): ReactNode {
  const out: ReactNode[] = [];
  // Order matters: inline code first so `**` inside backticks is literal.
  const re =
    /`([^`]+)`|\*\*([^*]+)\*\*|\*([^*\s][^*]*)\*|\[([^\]]+)\]\(([^)]+)\)/g;
  let last = 0;
  let key = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) out.push(text.slice(last, m.index));
    if (m[1] !== undefined) {
      out.push(
        <code
          key={key++}
          className="rounded bg-[var(--muted)] px-1.5 py-0.5 font-mono text-[0.85em]"
        >
          {m[1]}
        </code>,
      );
    } else if (m[2] !== undefined) {
      out.push(<strong key={key++}>{m[2]}</strong>);
    } else if (m[3] !== undefined) {
      out.push(<em key={key++}>{m[3]}</em>);
    } else if (m[4] !== undefined && m[5] !== undefined) {
      out.push(
        <a
          key={key++}
          href={m[5]}
          className="text-[var(--brand-primary)] underline"
        >
          {m[4]}
        </a>,
      );
    }
    last = re.lastIndex;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

/**
 * Strip comments the ported bodies carry: MDX block comments
 * (`{/* ... *​/}`) from the old `.mdx` blog posts, and HTML comments
 * (`<!-- ... -->`) from the legal/DPA template.
 */
function stripComments(src: string): string {
  return src
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/<!--[\s\S]*?-->/g, "");
}

function parse(src: string): Block[] {
  const lines = stripComments(src).replace(/\r\n/g, "\n").split("\n");
  const blocks: Block[] = [];
  let paragraph: string[] = [];
  let list: { kind: "ul" | "ol"; items: string[] } | null = null;
  let fence: string[] | null = null;

  const flush = () => {
    if (paragraph.length > 0) {
      blocks.push({ kind: "p", text: paragraph.join(" ").trim() });
      paragraph = [];
    }
    if (list) {
      blocks.push(list);
      list = null;
    }
  };

  for (const raw of lines) {
    // Fenced code block — capture verbatim until the closing fence.
    if (raw.trim().startsWith("```")) {
      if (fence === null) {
        flush();
        fence = [];
      } else {
        blocks.push({ kind: "code", text: fence.join("\n") });
        fence = null;
      }
      continue;
    }
    if (fence !== null) {
      fence.push(raw);
      continue;
    }

    const line = raw.trim();
    if (!line) {
      flush();
      continue;
    }
    if (line.startsWith("### ")) {
      flush();
      blocks.push({ kind: "h3", text: line.slice(4) });
      continue;
    }
    if (line.startsWith("## ")) {
      flush();
      blocks.push({ kind: "h2", text: line.slice(3) });
      continue;
    }
    if (line.startsWith("# ")) {
      flush();
      blocks.push({ kind: "h1", text: line.slice(2) });
      continue;
    }
    const ul = /^[-*] (.+)$/.exec(line);
    if (ul) {
      if (paragraph.length > 0) flush();
      if (!list || list.kind !== "ul") {
        if (list) flush();
        list = { kind: "ul", items: [] };
      }
      list.items.push(ul[1]);
      continue;
    }
    const ol = /^\d+\. (.+)$/.exec(line);
    if (ol) {
      if (paragraph.length > 0) flush();
      if (!list || list.kind !== "ol") {
        if (list) flush();
        list = { kind: "ol", items: [] };
      }
      list.items.push(ol[1]);
      continue;
    }
    if (list) flush();
    paragraph.push(line);
  }
  if (fence !== null) blocks.push({ kind: "code", text: fence.join("\n") });
  flush();
  return blocks;
}

/**
 * Render a markdown string to styled React nodes. Used by the ported
 * `/blog` posts and `/dpa` legal page.
 */
export function Markdown({ source }: { source: string }): ReactElement {
  const blocks = parse(source);
  return (
    <div className="flex flex-col gap-4">
      {blocks.map((b, i) => {
        switch (b.kind) {
          case "h1":
            return (
              <h1
                key={i}
                className="text-3xl sm:text-4xl font-bold tracking-tight text-[var(--foreground)]"
              >
                {renderInline(b.text)}
              </h1>
            );
          case "h2":
            return (
              <h2
                key={i}
                className="text-2xl font-semibold tracking-tight text-[var(--foreground)] mt-8"
              >
                {renderInline(b.text)}
              </h2>
            );
          case "h3":
            return (
              <h3
                key={i}
                className="text-lg font-semibold text-[var(--foreground)] mt-4"
              >
                {renderInline(b.text)}
              </h3>
            );
          case "ul":
            return (
              <ul
                key={i}
                className="list-disc pl-6 flex flex-col gap-2 text-[var(--muted-foreground)] leading-relaxed"
              >
                {b.items.map((it, j) => (
                  <li key={j}>{renderInline(it)}</li>
                ))}
              </ul>
            );
          case "ol":
            return (
              <ol
                key={i}
                className="list-decimal pl-6 flex flex-col gap-2 text-[var(--muted-foreground)] leading-relaxed"
              >
                {b.items.map((it, j) => (
                  <li key={j}>{renderInline(it)}</li>
                ))}
              </ol>
            );
          case "code":
            return (
              <pre
                key={i}
                className="my-2 overflow-x-auto rounded-xl border border-[var(--border)] bg-[var(--card)] p-4 font-mono text-sm text-[var(--foreground)]"
              >
                <code>{b.text}</code>
              </pre>
            );
          case "p":
          default:
            return (
              <p
                key={i}
                className="text-[var(--muted-foreground)] leading-relaxed"
              >
                {renderInline(b.text)}
              </p>
            );
        }
      })}
    </div>
  );
}
