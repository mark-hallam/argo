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
}

interface ProvisionPayload {
  workspace_name: string;
  company_name: string;
}

interface ProvisionResponse {
  tenant_id: string;
  workspace_id: string;
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
  return { tenantId: raw.tenant_id, workspaceId: raw.workspace_id };
}

export function useProvision() {
  return useMutation<ProvisionResult, Error, ProvisionRequest>({
    mutationFn: provision,
  });
}
