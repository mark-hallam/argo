"use client";

import { useCallback, useEffect, useRef, useState } from "react";

/**
 * `useTestRunHistory` — localStorage-backed log of recent builder test
 * runs, scoped per workflow.
 *
 * v1 fallback: gateway-side persistence lands in a follow-up brief.
 * Until then, the panel below the canvas reads from this hook so the
 * user can re-open completed runs after the overlay dismisses.
 *
 * History caps at MAX_ENTRIES (10) and evicts FIFO so the oldest run
 * drops first. The store key is namespaced by workflow so multiple
 * tabs editing different workflows do not collide.
 */

export const MAX_ENTRIES = 10;
const STORAGE_VERSION = 1;
const STORAGE_PREFIX = "skycot:test-run-history:v1";

export type TestRunHistoryStatus = "complete" | "failed";

export interface TestRunHistoryStepState {
  status: "pending" | "running" | "success" | "error";
  durationMs?: number;
  tokens?: number;
  response?: string;
  error?: string;
}

export interface TestRunHistoryStep {
  id: string;
  name: string;
  kind: string;
}

export interface TestRunHistorySummary {
  ok: number;
  fail: number;
  durationMs: number;
  tokens?: number;
  costUsd?: number;
}

export interface TestRunHistoryEntry {
  id: string;
  startedAt: number;
  endedAt: number;
  status: TestRunHistoryStatus;
  transportError?: string;
  steps: TestRunHistoryStep[];
  stepStates: Record<string, TestRunHistoryStepState>;
  summary: TestRunHistorySummary | null;
}

interface StoredShape {
  version: number;
  entries: TestRunHistoryEntry[];
}

function storageKey(workflowId: string): string {
  return `${STORAGE_PREFIX}:${workflowId}`;
}

function safeRead(workflowId: string): TestRunHistoryEntry[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(storageKey(workflowId));
    if (!raw) return [];
    const parsed = JSON.parse(raw) as Partial<StoredShape>;
    if (!parsed || parsed.version !== STORAGE_VERSION) return [];
    if (!Array.isArray(parsed.entries)) return [];
    return parsed.entries.slice(0, MAX_ENTRIES);
  } catch {
    return [];
  }
}

function safeWrite(workflowId: string, entries: TestRunHistoryEntry[]): void {
  if (typeof window === "undefined") return;
  try {
    const payload: StoredShape = { version: STORAGE_VERSION, entries };
    window.localStorage.setItem(storageKey(workflowId), JSON.stringify(payload));
  } catch {
    // Quota errors / Safari private mode — swallow so the run UI keeps
    // working; the history panel just won't persist this run.
  }
}

/**
 * Apply FIFO cap: newest entries live at index 0; truncate the tail.
 * Pure helper, exported for the unit test that asserts 12-in / 10-out.
 */
export function applyCap(entries: TestRunHistoryEntry[]): TestRunHistoryEntry[] {
  if (entries.length <= MAX_ENTRIES) return entries;
  return entries.slice(0, MAX_ENTRIES);
}

export interface UseTestRunHistoryResult {
  entries: TestRunHistoryEntry[];
  record: (entry: TestRunHistoryEntry) => void;
  clear: () => void;
}

export function useTestRunHistory(workflowId: string): UseTestRunHistoryResult {
  const [entries, setEntries] = useState<TestRunHistoryEntry[]>(() =>
    safeRead(workflowId),
  );
  // Mirror entries in a ref so callbacks compute the next snapshot
  // outside the setState updater. Doing storage IO inside the updater
  // makes a fast record+clear sequence write in the wrong order under
  // React batching.
  const entriesRef = useRef(entries);
  entriesRef.current = entries;

  useEffect(() => {
    const fresh = safeRead(workflowId);
    entriesRef.current = fresh;
    setEntries(fresh);
  }, [workflowId]);

  const record = useCallback(
    (entry: TestRunHistoryEntry) => {
      const next = applyCap([entry, ...entriesRef.current]);
      entriesRef.current = next;
      setEntries(next);
      safeWrite(workflowId, next);
    },
    [workflowId],
  );

  const clear = useCallback(() => {
    entriesRef.current = [];
    setEntries([]);
    safeWrite(workflowId, []);
  }, [workflowId]);

  return { entries, record, clear };
}
