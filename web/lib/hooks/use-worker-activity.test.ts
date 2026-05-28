import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import {
  WORKER_ACTIVITY_POLL_MS,
  WORKER_ACTIVITY_STALE_MS,
  useWorkerActivity,
} from "./use-worker-activity";
import type { WorkerActivityFilters } from "@/lib/api/worker-activity";

/**
 * Polling behaviour is verified by mocking `useInfiniteQuery` and asserting
 * the options object the hook hands to TanStack. This avoids fake-timer
 * coordination against the React Query internal scheduler — which is
 * environment-fragile in happy-dom — while still pinning the contract:
 * `refetchInterval` / `refetchIntervalInBackground` / `staleTime` MUST
 * flow through to the underlying query so the Scene 2 demo narration
 * ("the activity feed streams in real time") is supported.
 */

const observedOptions: Array<Record<string, unknown>> = [];

vi.mock("@tanstack/react-query", async () => {
  const actual = await vi.importActual<typeof import("@tanstack/react-query")>(
    "@tanstack/react-query",
  );
  return {
    ...actual,
    useInfiniteQuery: (options: Record<string, unknown>) => {
      observedOptions.push(options);
      return {
        data: undefined,
        isPending: true,
        isSuccess: false,
        isError: false,
        error: null,
      };
    },
  };
});

const FILTERS: WorkerActivityFilters = { range: "today" };

function lastOptions(): Record<string, unknown> {
  return observedOptions[observedOptions.length - 1];
}

describe("useWorkerActivity options", () => {
  it("constants match the v1.0 demo runbook 'watch this' beat (3s poll, 1s stale)", () => {
    expect(WORKER_ACTIVITY_POLL_MS).toBe(3000);
    expect(WORKER_ACTIVITY_STALE_MS).toBe(1000);
    expect(WORKER_ACTIVITY_STALE_MS).toBeLessThan(WORKER_ACTIVITY_POLL_MS);
  });

  it("default (live omitted) sets refetchInterval to the 3s polling cadence", () => {
    renderHook(() => useWorkerActivity("wkr_priya", FILTERS));
    const opts = lastOptions();
    expect(opts.refetchInterval).toBe(WORKER_ACTIVITY_POLL_MS);
  });

  it("live=true sets refetchInterval to the 3s polling cadence", () => {
    renderHook(() => useWorkerActivity("wkr_priya", FILTERS, { live: true }));
    const opts = lastOptions();
    expect(opts.refetchInterval).toBe(WORKER_ACTIVITY_POLL_MS);
  });

  it("live=false disables polling — opt-out for audit-drill-in consumers", () => {
    renderHook(() => useWorkerActivity("wkr_priya", FILTERS, { live: false }));
    const opts = lastOptions();
    expect(opts.refetchInterval).toBe(false);
  });

  it("pauses polling when the tab is hidden (refetchIntervalInBackground=false)", () => {
    renderHook(() => useWorkerActivity("wkr_priya", FILTERS));
    const opts = lastOptions();
    expect(opts.refetchIntervalInBackground).toBe(false);
  });

  it("staleTime 1s keeps tab-focus refetch from immediately re-firing", () => {
    renderHook(() => useWorkerActivity("wkr_priya", FILTERS));
    const opts = lastOptions();
    expect(opts.staleTime).toBe(WORKER_ACTIVITY_STALE_MS);
  });

  it("enabled defaults to true and is overridable", () => {
    renderHook(() => useWorkerActivity("wkr_priya", FILTERS));
    expect(lastOptions().enabled).toBe(true);

    renderHook(() => useWorkerActivity("wkr_priya", FILTERS, { enabled: false }));
    expect(lastOptions().enabled).toBe(false);
  });
});
