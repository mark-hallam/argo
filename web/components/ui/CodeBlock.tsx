"use client";

import { Check, Copy } from "lucide-react";
import { useState, type HTMLAttributes } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

export interface CodeBlockProps extends HTMLAttributes<HTMLPreElement> {
  code: string;
  language?: string;
  showCopy?: boolean;
}

export function CodeBlock({
  className,
  code,
  language,
  showCopy = true,
  ...props
}: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  async function onCopy() {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      // clipboard blocked — leave state alone
    }
  }
  return (
    <div
      data-testid="code-block"
      className={cn(
        "relative rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)] text-[var(--foreground)] overflow-hidden",
        className,
      )}
    >
      {(language || showCopy) ? (
        <div className="flex items-center justify-between border-b border-[var(--border)] px-3 py-2 bg-[var(--card)]/40">
          {language ? (
            <code className="text-xs font-mono text-[var(--muted-foreground)] uppercase tracking-wide">
              {language}
            </code>
          ) : <span />}
          {showCopy ? (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={onCopy}
              aria-label={copied ? "Copied" : "Copy code"}
              data-testid="code-block-copy"
            >
              {copied ? <Check aria-hidden /> : <Copy aria-hidden />}
              <span className="text-xs">{copied ? "Copied" : "Copy"}</span>
            </Button>
          ) : null}
        </div>
      ) : null}
      <pre
        {...props}
        className={cn(
          "overflow-x-auto px-4 py-3 text-sm font-mono leading-relaxed",
        )}
      >
        <code>{code}</code>
      </pre>
    </div>
  );
}
