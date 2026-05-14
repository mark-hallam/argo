"use client";

/**
 * Server-defined approval-bundle hooks (mission-approval-bundle-ui).
 *
 * The backend exposes:
 *   - GET    /api/v1/approvals/bundles?status=pending
 *   - POST   /api/v1/approvals/bundles
 *   - POST   /api/v1/approvals/bundles/<id>/decide
 *
 * Wire shapes here are deliberately narrow — the page wires bundles
 * as a separate above-the-fold rail; if the gateway returns 404 (J1
 * shipped before this brief) or the connection fails, we fall back
 * to an empty list so the rail simply doesn't render.
 *
 * The frontend client lives inline here to keep the brief's
 * may-touch list tight; the existing approvals client at
 * `web/lib/api/approvals.ts` is the model for the fetch + fallback
 * shape.
 */

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

const BUNDLES_KEY = ["approval-bundles"] as const;
const APPROVALS_KEY = ["approvals"] as const;

export type BundleStatus = "pending" | "approved" | "refused";

/**
 * Bundle row as rendered in the UI. Matches the BundleCard prop
 * shape; the gateway wire shape is `BundleRowJson` below.
 */
export interface ApprovalBundle {
  bundleId: string;
  label: string;
  count: number;
  members: string[];
  createdAt: number;
  status: BundleStatus;
  decision?: "approve" | "refuse" | null;
  decidedAt?: number | null;
  decidedBy?: string | null;
}

export interface BundleDecideRequest {
  bundleId: string;
  decision: "approve" | "refuse";
}

export interface BundleDecideResult {
  ok: true;
  bundleId: string;
  decision: "approve" | "refuse";
  membersDecided: number;
  membersSkipped: number;
}

interface BundleRowJson {
  bundle_id: string;
  label: string;
  count: number;
  members: string[];
  created_at: number;
  status: BundleStatus;
  decision?: "approve" | "refuse" | null;
  decided_at?: number | null;
  decided_by?: string | null;
}

interface BundlesListJson {
  items: BundleRowJson[];
}

function rowToBundle(row: BundleRowJson): ApprovalBundle {
  return {
    bundleId: row.bundle_id,
    label: row.label,
    count: row.count,
    members: row.members,
    createdAt: row.created_at,
    status: row.status,
    decision: row.decision ?? null,
    decidedAt: row.decided_at ?? null,
    decidedBy: row.decided_by ?? null,
  };
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

function authBypassEnabled(): boolean {
  return process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1";
}

export async function fetchApprovalBundles(): Promise<ApprovalBundle[]> {
  if (authBypassEnabled()) {
    return [];
  }
  try {
    const raw = await apiGet<BundlesListJson | BundleRowJson[]>(
      "/api/v1/approvals/bundles?status=pending",
    );
    const rows = Array.isArray(raw) ? raw : raw.items;
    return rows.map(rowToBundle);
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      return [];
    }
    throw err;
  }
}

export async function submitBundleDecision({
  bundleId,
  decision,
}: BundleDecideRequest): Promise<BundleDecideResult> {
  const res = await apiPost<{
    ok: true;
    bundle_id: string;
    decision: "approve" | "refuse";
    members_decided: number;
    members_skipped: number;
  }>(`/api/v1/approvals/bundles/${bundleId}/decide`, { decision });
  return {
    ok: true,
    bundleId: res.bundle_id ?? bundleId,
    decision: res.decision ?? decision,
    membersDecided: res.members_decided ?? 0,
    membersSkipped: res.members_skipped ?? 0,
  };
}

/** Pending bundles for the current workspace tenant. */
export function useApprovalBundles() {
  return useQuery({
    queryKey: BUNDLES_KEY,
    queryFn: fetchApprovalBundles,
  });
}

/**
 * Decide a bundle. Optimistically removes it from the pending list;
 * invalidates the per-approval cache too because the cascade flips
 * member ApprovalCard states server-side.
 */
export function useDecideBundle() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: submitBundleDecision,
    onMutate: async ({ bundleId }: BundleDecideRequest) => {
      await qc.cancelQueries({ queryKey: BUNDLES_KEY });
      const prev = qc.getQueryData<ApprovalBundle[]>(BUNDLES_KEY);
      if (prev) {
        qc.setQueryData<ApprovalBundle[]>(
          BUNDLES_KEY,
          prev.filter((b) => b.bundleId !== bundleId),
        );
      }
      return { prev };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.prev) qc.setQueryData(BUNDLES_KEY, ctx.prev);
    },
    onSettled: () => {
      void qc.invalidateQueries({ queryKey: BUNDLES_KEY });
      void qc.invalidateQueries({ queryKey: APPROVALS_KEY });
    },
  });
}
