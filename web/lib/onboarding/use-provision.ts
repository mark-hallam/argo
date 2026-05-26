"use client";

import { useMutation } from "@tanstack/react-query";
import { apiPost, ApiError } from "@/lib/api/client";

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

function isProvisionResponse(value: unknown): value is ProvisionResponse {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return typeof v.tenant_id === "string" && typeof v.workspace_id === "string";
}

export async function provision(req: ProvisionRequest): Promise<ProvisionResult> {
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
