"use client";

import { useMutation } from "@tanstack/react-query";
import { apiPost, ApiError } from "@/lib/api/client";
import { isDemoMode } from "@/lib/skycot/demo-mode";

export interface ProvisionRequest {
  workspaceName: string;
  companyName: string;
}

export interface ProvisionResult {
  tenantId: string;
  workspaceId: string;
  // `true` when this call freshly provisioned the workspace; `false`
  // on an idempotent re-call. Drives the workspace-step celebration
  // toast — older gateway builds that omit the field are treated as
  // idempotent so we never falsely celebrate a re-call.
  created: boolean;
}

interface ProvisionPayload {
  workspace_name: string;
  company_name: string;
}

interface ProvisionResponse {
  tenant_id: string;
  workspace_id: string;
  created?: boolean;
}

// Shared sessionStorage keys so any wizard step downstream of `/workspace`
// can read the demo tenant/workspace ids without re-deriving them.
export const DEMO_TENANT_STORAGE_KEY = "skycot:demo-onboarding-tenant-id";
export const DEMO_WORKSPACE_STORAGE_KEY = "skycot:demo-onboarding-workspace-id";

function isProvisionResponse(value: unknown): value is ProvisionResponse {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return typeof v.tenant_id === "string" && typeof v.workspace_id === "string";
}

function slugifyForId(input: string): string {
  const lowered = input.trim().toLowerCase();
  const slug = lowered.replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");
  return slug.length > 0 ? slug.slice(0, 40) : "workspace";
}

/**
 * Deterministic synthetic IDs for the demo wizard. The id is stable
 * per workspace name within a session so that subsequent steps (which
 * read from sessionStorage / the persisted draft) see the same tenant
 * the workspace step "provisioned." No gateway round-trip happens.
 */
function buildDemoProvisionResult(workspaceName: string): ProvisionResult {
  const slug = slugifyForId(workspaceName);
  return {
    tenantId: `demo-tenant-${slug}`,
    workspaceId: `demo-workspace-${slug}`,
  };
}

function persistDemoIds(result: ProvisionResult): void {
  if (typeof window === "undefined") return;
  try {
    window.sessionStorage.setItem(DEMO_TENANT_STORAGE_KEY, result.tenantId);
    window.sessionStorage.setItem(DEMO_WORKSPACE_STORAGE_KEY, result.workspaceId);
  } catch {
    // sessionStorage can throw in locked-down browser modes; the
    // wizard's persistent zustand store already has the ids, so we
    // swallow rather than block onboarding on a storage failure.
  }
}

export async function provision(req: ProvisionRequest): Promise<ProvisionResult> {
  // Demo-mode short-circuit (F5/F5a from chrome-live-walk-and-playwright
  // 2026-05-26-2007): the investor-pitch demo runs without a real
  // human:<email> principal, but /api/v1/onboarding/provision requires
  // one. Skip the gateway round-trip entirely and return synthetic ids
  // so the wizard can walk end-to-end in demo mode.
  if (isDemoMode()) {
    const result = buildDemoProvisionResult(req.workspaceName);
    persistDemoIds(result);
    return result;
  }
  const raw = await apiPost<unknown, ProvisionPayload>(
    "/api/v1/onboarding/provision",
    { workspace_name: req.workspaceName, company_name: req.companyName },
  );
  if (!isProvisionResponse(raw)) {
    throw new ApiError(502, "provision endpoint returned an unexpected shape");
  }
  return {
    tenantId: raw.tenant_id,
    workspaceId: raw.workspace_id,
    created: raw.created === true,
  };
}

export function useProvision() {
  return useMutation<ProvisionResult, Error, ProvisionRequest>({
    mutationFn: provision,
  });
}
