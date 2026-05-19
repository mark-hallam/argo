/**
 * Line-level unified diff between two strings, suitable for rendering
 * a workflow-version comparison view.
 *
 * Self-contained — no external diff dependency. Uses a Hunt–Szymanski
 * style longest-common-subsequence pass and groups the resulting edit
 * script into hunks with conventional unified-diff context.
 *
 * Output is RSC-safe (pure function, no DOM / no Node APIs) so the
 * computation can run at request time on the edge.
 */

export type DiffLineKind = "context" | "added" | "removed";

export interface DiffLine {
  kind: DiffLineKind;
  /** 1-indexed line in the "from" source, undefined for added lines. */
  fromLine?: number;
  /** 1-indexed line in the "to" source, undefined for removed lines. */
  toLine?: number;
  text: string;
}

export interface DiffHunk {
  fromStart: number;
  fromLength: number;
  toStart: number;
  toLength: number;
  lines: DiffLine[];
}

export interface UnifiedDiff {
  hunks: DiffHunk[];
  /** True when `from` and `to` produce no add/remove operations. */
  unchanged: boolean;
}

const DEFAULT_CONTEXT = 3;

export function compute_unified(
  from: string,
  to: string,
  context: number = DEFAULT_CONTEXT,
): UnifiedDiff {
  const fromLines = splitLines(from);
  const toLines = splitLines(to);
  const ops = lcsDiff(fromLines, toLines);
  const unchanged = ops.every((op) => op.kind === "context");
  if (unchanged) return { hunks: [], unchanged: true };
  return { hunks: groupHunks(ops, context), unchanged: false };
}

function splitLines(text: string): string[] {
  if (text.length === 0) return [];
  const lines = text.split(/\r?\n/);
  if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}

interface Op {
  kind: DiffLineKind;
  fromLine?: number;
  toLine?: number;
  text: string;
}

function lcsDiff(a: string[], b: string[]): Op[] {
  const n = a.length;
  const m = b.length;
  const dp: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      if (a[i] === b[j]) dp[i][j] = dp[i + 1][j + 1] + 1;
      else dp[i][j] = Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }
  const ops: Op[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      ops.push({ kind: "context", fromLine: i + 1, toLine: j + 1, text: a[i] });
      i++;
      j++;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      ops.push({ kind: "removed", fromLine: i + 1, text: a[i] });
      i++;
    } else {
      ops.push({ kind: "added", toLine: j + 1, text: b[j] });
      j++;
    }
  }
  while (i < n) {
    ops.push({ kind: "removed", fromLine: i + 1, text: a[i] });
    i++;
  }
  while (j < m) {
    ops.push({ kind: "added", toLine: j + 1, text: b[j] });
    j++;
  }
  return ops;
}

function groupHunks(ops: Op[], context: number): DiffHunk[] {
  const hunks: DiffHunk[] = [];
  let cursor = 0;
  while (cursor < ops.length) {
    const changeAt = findNextChange(ops, cursor);
    if (changeAt === -1) break;
    const hunkStart = Math.max(0, changeAt - context);
    let hunkEnd = changeAt;
    while (hunkEnd < ops.length) {
      const nextChange = findNextChange(ops, hunkEnd + 1);
      if (nextChange === -1 || nextChange - hunkEnd > context * 2) {
        hunkEnd = Math.min(ops.length - 1, lastChangeIn(ops, hunkEnd, nextChange) + context);
        break;
      }
      hunkEnd = nextChange;
    }
    hunkEnd = Math.min(ops.length - 1, hunkEnd);
    hunks.push(buildHunk(ops.slice(hunkStart, hunkEnd + 1)));
    cursor = hunkEnd + 1;
  }
  return hunks;
}

function findNextChange(ops: Op[], from: number): number {
  for (let k = from; k < ops.length; k++) {
    if (ops[k].kind !== "context") return k;
  }
  return -1;
}

function lastChangeIn(ops: Op[], from: number, fallback: number): number {
  let last = from;
  const upper = fallback === -1 ? ops.length : fallback;
  for (let k = from; k < upper; k++) {
    if (ops[k].kind !== "context") last = k;
  }
  return last;
}

function buildHunk(window: Op[]): DiffHunk {
  let fromStart = 0;
  let toStart = 0;
  let fromLength = 0;
  let toLength = 0;
  for (const op of window) {
    if (op.fromLine !== undefined && fromStart === 0) fromStart = op.fromLine;
    if (op.toLine !== undefined && toStart === 0) toStart = op.toLine;
    if (op.kind !== "added") fromLength++;
    if (op.kind !== "removed") toLength++;
  }
  return {
    fromStart: fromStart || 1,
    fromLength,
    toStart: toStart || 1,
    toLength,
    lines: window.map((op) => ({ ...op })),
  };
}
