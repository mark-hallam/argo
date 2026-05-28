"use client";

import { useSyncExternalStore } from "react";

/**
 * Canonical data-source signal consumed by `DemoBanner`. Each
 * `lib/api/*` fetcher maps its internal source discriminator onto one
 * of these three values and calls `publishDataSource()` after every
 * resolution; the banner subscribes via `useDataSource()` and branches
 * its copy so a viewer pausing on `/audit` against the live (empty)
 * gateway no longer reads "showing sample data" — the banner now says
 * the truth for the source actually feeding the page.
 *
 *  - `"live"`         — real gateway answered (any non-mock path,
 *                       including empty rows + `team.ts`'s
 *                       `"unreachable"` empty-state).
 *  - `"mock-offline"` — gateway 404/network-error fall-through to the
 *                       seed fixtures (`SAMPLE_*`). The banner says
 *                       "backend unreachable, showing seed data".
 *  - `"mock-bypass"`  — `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`
 *                       short-circuited before any gateway request
 *                       left the browser. The banner says "showing
 *                       curated sample data".
 */
export type DataSource = "live" | "mock-offline" | "mock-bypass";

let current: DataSource | null = null;
const listeners = new Set<() => void>();

export function publishDataSource(source: DataSource): void {
  if (current === source) return;
  current = source;
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function getSnapshot(): DataSource | null {
  return current;
}

/**
 * SSR returns `null` so the banner renders the unbranded default copy
 * during server-render; the first client publication after hydrate
 * triggers a re-render with the source-aware copy. The store is a
 * module-scoped singleton, so SSR-side `current` is whatever the last
 * Node worker happened to set — `getServerSnapshot` masks that for
 * correctness.
 */
function getServerSnapshot(): DataSource | null {
  return null;
}

export function useDataSource(): DataSource | null {
  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
}

/** Test-only reset hook. The module-scoped `current` survives across
 *  individual tests; vitest can call this in `beforeEach` to start
 *  from a clean slate. Not exported via barrel; tests import directly. */
export function __resetDataSourceForTests(): void {
  current = null;
  for (const listener of listeners) listener();
}
