/**
 * Tenant-scoped dollar-spend fetchers for the billing + worker
 * surfaces (UF A7, DEC-038).
 *
 * Reads two live, tenant-scoped endpoints
 * (`crates/argo-cli/src/handlers/gateway/routes_tenant_costs.rs`):
 *
 *   - `GET /api/v1/tenant/costs/total`     → `{spent_usd_micros}`
 *   - `GET /api/v1/tenant/costs/by-worker` → `{by_worker: [...]}`
 *
 * Both bind `WHERE tenant_id = ?`, so the numbers are this tenant's
 * spend only — tenant A never sees tenant B. Modelled on
 * `web/lib/api/usage.ts`.
 *
 * NO THEATER: amounts are USD micros straight from `spend_ledger`;
 * `worker_id` is `null` for system / non-teammate spend. Nothing is
 * fabricated client-side.
 */

import { apiGet } from "@/lib/api/client";

/** Live response from `GET /api/v1/tenant/costs/total`. */
export interface TenantCostTotal {
  /** All-time tenant spend in USD micros (1e6 micros = $1). */
  spent_usd_micros: number;
}

/** One row of `GET /api/v1/tenant/costs/by-worker`. */
export interface WorkerCost {
  /** Canonical worker UUID (matches `/api/v1/workers` ids), or `null`
   *  for system / non-teammate spend. */
  worker_id: string | null;
  /**
   * Server-resolved teammate name (joined from the worker store on the
   * gateway). `null` when the row is system spend (no `worker_id`) or the
   * id resolves to no current worker. Preferring this over a client-side
   * roster join is what fixes the "Unknown teammate" label for Genesis
   * hires (shakedown F11) — the roster fetch can fall back to a demo
   * sample or omit archived workers, but this name is always authoritative.
   * Additive: absent on older gateways (treat as `undefined`).
   */
  worker_name?: string | null;
  /** This teammate's spend in USD micros. */
  spent_usd_micros: number;
}

/** Live response from `GET /api/v1/tenant/costs/by-worker`. */
export interface TenantCostByWorker {
  by_worker: WorkerCost[];
}

/** Fetch the active tenant's all-time $ spend from the live endpoint. */
export function fetchTenantCostTotal(): Promise<TenantCostTotal> {
  return apiGet<TenantCostTotal>("/api/v1/tenant/costs/total");
}

/** Fetch the active tenant's $ spend grouped per AI teammate. */
export function fetchTenantCostByWorker(): Promise<TenantCostByWorker> {
  return apiGet<TenantCostByWorker>("/api/v1/tenant/costs/by-worker");
}

/**
 * Format USD micros as a dollar string. Uses 4 decimal places below
 * a cent so sub-cent teammate costs stay legible, 2 places at or above
 * a dollar. `0` renders as `$0.00`, never a blank.
 */
export function formatUsdMicros(micros: number): string {
  if (!Number.isFinite(micros) || micros <= 0) return "$0.00";
  const usd = micros / 1_000_000;
  const places = usd < 1 ? 4 : 2;
  return `$${usd.toFixed(places)}`;
}
