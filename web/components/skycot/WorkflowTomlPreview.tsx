"use client";

import { Check, Copy } from "lucide-react";
import { useState, useMemo } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

/**
 * Minimal TOML syntax highlighter for the proposed-workflow preview.
 * Renders headers (`[workflow]`, `[[step]]`), keys, strings, numbers,
 * booleans, and comments with distinct colors. Falls back to plain
 * text for anything it can't classify so streamed partial output is
 * always safe to render. Builds on shared design tokens so it picks
 * up the operator's theme automatically.
 */
export interface WorkflowTomlPreviewProps {
  toml: string;
  /** When true, render a copy-to-clipboard button in the header row. */
  showCopy?: boolean;
  /** When true, render the live status pill (used during streaming). */
  isStreaming?: boolean;
  className?: string;
}

interface HighlightedLine {
  key: number;
  tokens: HighlightedToken[];
}

interface HighlightedToken {
  kind:
    | "comment"
    | "table"
    | "table-array"
    | "key"
    | "string"
    | "number"
    | "boolean"
    | "punct"
    | "text";
  text: string;
}

const STRING_RE = /^"(?:\\.|[^"\\])*"|^'[^']*'/;
const NUMBER_RE = /^-?\d+(?:\.\d+)?/;

function tokenizeLine(line: string): HighlightedToken[] {
  const trimmed = line.trimStart();
  if (trimmed.startsWith("#")) {
    return [{ kind: "comment", text: line }];
  }
  if (/^\s*\[\[.+\]\]\s*$/.test(line)) {
    return [{ kind: "table-array", text: line }];
  }
  if (/^\s*\[.+\]\s*$/.test(line)) {
    return [{ kind: "table", text: line }];
  }
  const tokens: HighlightedToken[] = [];
  const eqIdx = line.indexOf("=");
  let cursor = 0;
  if (eqIdx > 0) {
    const leading = line.slice(0, eqIdx);
    const keyMatch = /^(\s*)([A-Za-z0-9_.-]+)(\s*)$/.exec(leading);
    if (keyMatch) {
      if (keyMatch[1]) tokens.push({ kind: "text", text: keyMatch[1] });
      tokens.push({ kind: "key", text: keyMatch[2]! });
      if (keyMatch[3]) tokens.push({ kind: "text", text: keyMatch[3] });
      tokens.push({ kind: "punct", text: "=" });
      cursor = eqIdx + 1;
    }
  }
  while (cursor < line.length) {
    const remainder = line.slice(cursor);
    if (remainder.startsWith(" ") || remainder.startsWith("\t")) {
      const ws = remainder.match(/^\s+/)![0];
      tokens.push({ kind: "text", text: ws });
      cursor += ws.length;
      continue;
    }
    const strMatch = STRING_RE.exec(remainder);
    if (strMatch) {
      tokens.push({ kind: "string", text: strMatch[0] });
      cursor += strMatch[0].length;
      continue;
    }
    const numMatch = NUMBER_RE.exec(remainder);
    if (numMatch) {
      tokens.push({ kind: "number", text: numMatch[0] });
      cursor += numMatch[0].length;
      continue;
    }
    if (remainder.startsWith("true") || remainder.startsWith("false")) {
      const word = remainder.startsWith("true") ? "true" : "false";
      tokens.push({ kind: "boolean", text: word });
      cursor += word.length;
      continue;
    }
    if ("{}[],=".includes(remainder[0]!)) {
      tokens.push({ kind: "punct", text: remainder[0]! });
      cursor += 1;
      continue;
    }
    const wordMatch = /^[^\s{}\[\],"'=]+/.exec(remainder);
    if (wordMatch) {
      tokens.push({ kind: "text", text: wordMatch[0] });
      cursor += wordMatch[0].length;
      continue;
    }
    tokens.push({ kind: "text", text: remainder[0]! });
    cursor += 1;
  }
  return tokens;
}

function highlight(toml: string): HighlightedLine[] {
  return toml.split("\n").map((line, idx) => ({
    key: idx,
    tokens: tokenizeLine(line),
  }));
}

const KIND_CLASS: Record<HighlightedToken["kind"], string> = {
  comment: "text-[color:var(--muted-foreground)] italic",
  table: "text-[color:var(--primary)] font-semibold",
  "table-array": "text-[color:var(--primary)] font-semibold",
  key: "text-[color:var(--foreground)] font-medium",
  string: "text-emerald-700 dark:text-emerald-300",
  number: "text-amber-700 dark:text-amber-300",
  boolean: "text-amber-700 dark:text-amber-300",
  punct: "text-[color:var(--muted-foreground)]",
  text: "text-[color:var(--foreground)]",
};

export function WorkflowTomlPreview({
  toml,
  showCopy = true,
  isStreaming = false,
  className,
}: WorkflowTomlPreviewProps) {
  const lines = useMemo(() => highlight(toml), [toml]);
  const [copied, setCopied] = useState(false);

  async function onCopy() {
    try {
      await navigator.clipboard.writeText(toml);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      // clipboard unavailable; leave state alone
    }
  }

  return (
    <div
      data-testid="workflow-toml-preview"
      className={cn(
        "relative rounded-[var(--radius-md)] border border-[var(--border)]",
        "bg-[var(--muted)] text-[var(--foreground)] overflow-hidden",
        className,
      )}
    >
      <div className="flex items-center justify-between border-b border-[var(--border)] px-3 py-2 bg-[var(--card)]/40">
        <code className="text-xs font-mono text-[color:var(--muted-foreground)] uppercase tracking-wide">
          {isStreaming ? "TOML · streaming" : "TOML"}
        </code>
        {showCopy ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={onCopy}
            disabled={toml.length === 0}
            aria-label={copied ? "Copied" : "Copy TOML"}
            data-testid="workflow-toml-copy"
          >
            {copied ? <Check aria-hidden /> : <Copy aria-hidden />}
            <span className="text-xs">{copied ? "Copied" : "Copy"}</span>
          </Button>
        ) : null}
      </div>
      <pre
        className="overflow-x-auto px-4 py-3 text-sm font-mono leading-relaxed"
        data-testid="workflow-toml-body"
      >
        <code>
          {lines.map((line, idx) => (
            <div key={line.key} data-line={idx}>
              {line.tokens.length === 0 ? (
                <span>{"​"}</span>
              ) : (
                line.tokens.map((tok, tIdx) => (
                  <span key={tIdx} className={KIND_CLASS[tok.kind]}>
                    {tok.text}
                  </span>
                ))
              )}
            </div>
          ))}
        </code>
      </pre>
    </div>
  );
}
