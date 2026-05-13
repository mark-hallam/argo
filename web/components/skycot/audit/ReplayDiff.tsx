import { useMemo } from "react";
import type { ReplayResult } from "@/lib/api/replay";

/**
 * Output diff card — side-by-side (lg+) or stacked (mobile) renderer
 * for the original vs replayed output. Text bodies are token-level
 * diffed with a simple LCS algorithm; JSON bodies are pretty-printed
 * and diffed the same way; computer-use bodies render two screenshot
 * grids side by side per the wireframe.
 *
 * Token tags:
 *   same   → plain text
 *   add    → success-tone background @ 12% alpha (only on replayed side)
 *   remove → destructive-tone background @ 12% alpha (only on original side)
 */

type Side = "original" | "replayed";

interface DiffToken {
  text: string;
  tag: "same" | "add" | "remove";
}

export interface ReplayDiffProps {
  result: ReplayResult;
}

export function ReplayDiff({ result }: ReplayDiffProps) {
  const originalKind = result.originalOutput.kind;
  const replayedKind = result.replayedOutput.kind;
  const isComputerUse =
    originalKind === "computer-use" || replayedKind === "computer-use";

  return (
    <section
      data-testid="replay-diff"
      data-original-kind={originalKind}
      data-replayed-kind={replayedKind}
      className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 space-y-3"
    >
      <header className="flex items-center justify-between">
        <h2 className="text-sm font-semibold text-[var(--foreground)]">
          Output diff
        </h2>
        <span className="text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
          {isComputerUse ? "Computer use" : "Text"}
        </span>
      </header>

      {isComputerUse ? (
        <ComputerUseDiff result={result} />
      ) : (
        <TextDiff result={result} />
      )}
    </section>
  );
}

function TextDiff({ result }: { result: ReplayResult }) {
  const original = stringifyBody(result.originalOutput.body);
  const replayed = stringifyBody(result.replayedOutput.body);

  const { originalTokens, replayedTokens } = useMemo(
    () => tokenLevelDiff(original, replayed),
    [original, replayed],
  );

  return (
    <div className="grid gap-3 lg:grid-cols-2">
      <DiffPane side="original" tokens={originalTokens} />
      <DiffPane side="replayed" tokens={replayedTokens} />
    </div>
  );
}

function DiffPane({ side, tokens }: { side: Side; tokens: DiffToken[] }) {
  return (
    <div
      data-testid={`replay-diff-pane-${side}`}
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/20 p-3"
    >
      <p className="mb-2 text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
        {side === "original" ? "Original" : "Replayed"}
      </p>
      <pre className="max-h-96 overflow-auto text-sm font-mono whitespace-pre-wrap break-words text-[var(--foreground)]">
        {tokens.map((tok, idx) => (
          <DiffSpan key={`${side}-${idx}`} side={side} token={tok} />
        ))}
      </pre>
    </div>
  );
}

function DiffSpan({ side, token }: { side: Side; token: DiffToken }) {
  if (token.tag === "same") return <span>{token.text}</span>;
  if (side === "original" && token.tag === "remove") {
    return (
      <span
        data-testid="replay-diff-token-remove"
        className="bg-[var(--destructive)]/[0.12] text-[var(--foreground)] rounded-sm"
      >
        {token.text}
      </span>
    );
  }
  if (side === "replayed" && token.tag === "add") {
    return (
      <span
        data-testid="replay-diff-token-add"
        className="bg-[var(--success)]/[0.12] text-[var(--foreground)] rounded-sm"
      >
        {token.text}
      </span>
    );
  }
  return null;
}

function ComputerUseDiff({ result }: { result: ReplayResult }) {
  const originalFrames = framesFrom(result.originalOutput.body);
  const replayedFrames = framesFrom(result.replayedOutput.body);
  return (
    <div className="grid gap-3 lg:grid-cols-2">
      <FrameGrid
        side="original"
        label="Original sequence"
        frames={originalFrames}
        compareCount={replayedFrames.length}
      />
      <FrameGrid
        side="replayed"
        label="Replayed sequence"
        frames={replayedFrames}
        compareCount={originalFrames.length}
      />
    </div>
  );
}

interface CuFrame {
  url: string;
  description?: string;
}

function framesFrom(body: unknown): CuFrame[] {
  if (Array.isArray(body)) {
    return body
      .filter((f): f is CuFrame => typeof f === "object" && f !== null && "url" in f)
      .map((f) => ({ url: f.url, description: f.description }));
  }
  if (typeof body === "object" && body !== null && "screenshots" in body) {
    const screenshots = (body as { screenshots?: unknown }).screenshots;
    if (Array.isArray(screenshots)) return framesFrom(screenshots);
  }
  return [];
}

function FrameGrid({
  side,
  label,
  frames,
  compareCount,
}: {
  side: Side;
  label: string;
  frames: CuFrame[];
  compareCount: number;
}) {
  return (
    <div
      data-testid={`replay-diff-frames-${side}`}
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/20 p-3"
    >
      <p className="mb-2 text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
        {label}
      </p>
      {frames.length === 0 ? (
        <p className="text-xs text-[var(--muted-foreground)]">
          No screenshots captured.
        </p>
      ) : (
        <ol className="grid grid-cols-2 gap-2 sm:grid-cols-3">
          {frames.map((frame, idx) => {
            const drift = idx >= compareCount;
            return (
              <li
                key={`${side}-${idx}`}
                data-testid={`replay-diff-frame-${side}-${idx}`}
              >
                <a
                  href={frame.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className={`block rounded-[var(--radius-sm)] border bg-[var(--card)] p-2 ${
                    drift
                      ? "border-[var(--warning)]/50"
                      : "border-[var(--border)] hover:border-[var(--primary)]/40"
                  }`}
                >
                  <img
                    src={frame.url}
                    alt={frame.description ?? `Frame ${idx + 1}`}
                    className="aspect-video w-full rounded-[var(--radius-sm)] object-cover bg-[var(--muted)]"
                    loading="lazy"
                  />
                  <p className="mt-1 text-[11px] text-[var(--muted-foreground)]">
                    {idx + 1}. {frame.description ?? "frame"}
                  </p>
                </a>
              </li>
            );
          })}
        </ol>
      )}
    </div>
  );
}

function stringifyBody(body: unknown): string {
  if (typeof body === "string") return body;
  try {
    return JSON.stringify(body, null, 2);
  } catch {
    return String(body);
  }
}

/** Split a string into diff-friendly tokens. Splits on whitespace
 *  boundaries while preserving the whitespace characters so the
 *  reassembled output looks like the original input. */
export function tokenize(input: string): string[] {
  const matches = input.match(/\s+|\S+/g);
  return matches ?? [];
}

/** Token-level diff via LCS. Returns paired token streams per side
 *  with `same` / `add` / `remove` tags. Exported for unit tests. */
export function tokenLevelDiff(
  original: string,
  replayed: string,
): { originalTokens: DiffToken[]; replayedTokens: DiffToken[] } {
  const a = tokenize(original);
  const b = tokenize(replayed);
  const lcs = lcsTable(a, b);

  const originalTokens: DiffToken[] = [];
  const replayedTokens: DiffToken[] = [];
  let i = a.length;
  let j = b.length;
  while (i > 0 && j > 0) {
    if (a[i - 1] === b[j - 1]) {
      originalTokens.unshift({ text: a[i - 1], tag: "same" });
      replayedTokens.unshift({ text: b[j - 1], tag: "same" });
      i--;
      j--;
    } else if (lcs[i - 1][j] >= lcs[i][j - 1]) {
      originalTokens.unshift({ text: a[i - 1], tag: "remove" });
      i--;
    } else {
      replayedTokens.unshift({ text: b[j - 1], tag: "add" });
      j--;
    }
  }
  while (i > 0) {
    originalTokens.unshift({ text: a[i - 1], tag: "remove" });
    i--;
  }
  while (j > 0) {
    replayedTokens.unshift({ text: b[j - 1], tag: "add" });
    j--;
  }
  return { originalTokens, replayedTokens };
}

function lcsTable(a: string[], b: string[]): number[][] {
  const rows = a.length + 1;
  const cols = b.length + 1;
  const table: number[][] = Array.from({ length: rows }, () => new Array(cols).fill(0));
  for (let i = 1; i < rows; i++) {
    for (let j = 1; j < cols; j++) {
      if (a[i - 1] === b[j - 1]) {
        table[i][j] = table[i - 1][j - 1] + 1;
      } else {
        table[i][j] = Math.max(table[i - 1][j], table[i][j - 1]);
      }
    }
  }
  return table;
}
