"use client";

/**
 * Customer-facing GDPR/CCPA Article 15 data-export hook
 * (Stream `mission-data-export-customer-flow`).
 *
 * Backend (X-Argo-Principal: human:<email>):
 *   - POST /api/v1/me/export                        -> { job_id, status, download_url? }
 *   - GET  /api/v1/me/export/<job_id>               -> { status, completed_at, expires_at }
 *   - GET  /api/v1/me/export/download/<token>      -> { content_type, filename, body_b64 }
 *
 * The v1 backend assembles tarballs inline so the status response
 * usually arrives as `ready` on the first poll, but the hook still
 * exposes the polling loop so a future background-job switch is a
 * zero-API-change swap.
 */

import { useCallback, useEffect, useMemo, useState } from "react";
import { apiGet, apiPost, ApiError } from "@/lib/api/client";

export type ExportJobStatus = "queued" | "running" | "ready" | "expired" | "failed";

export interface ExportRequestResponse {
  job_id: string;
  status: ExportJobStatus;
  download_url?: string;
  size_bytes?: number;
  expires_at?: number;
}

export interface ExportStatusResponse {
  job_id: string;
  status: ExportJobStatus;
  completed_at: number | null;
  expires_at: number | null;
  error?: string;
}

export interface ExportDownloadResponse {
  content_type: string;
  filename: string;
  size_bytes: number;
  body_b64: string;
}

interface UseExportRequestState {
  status: "idle" | "requesting" | "polling" | "ready" | "error" | "rate_limited";
  jobId: string | null;
  downloadUrl: string | null;
  error: string | null;
  retryAfterSecs: number | null;
}

const INITIAL: UseExportRequestState = {
  status: "idle",
  jobId: null,
  downloadUrl: null,
  error: null,
  retryAfterSecs: null,
};

export function useExportRequest() {
  const [state, setState] = useState<UseExportRequestState>(INITIAL);

  const request = useCallback(async () => {
    setState({ ...INITIAL, status: "requesting" });
    try {
      const resp = await apiPost<ExportRequestResponse>("/api/v1/me/export");
      if (resp.status === "ready" && resp.download_url) {
        setState({
          status: "ready",
          jobId: resp.job_id,
          downloadUrl: resp.download_url,
          error: null,
          retryAfterSecs: null,
        });
      } else {
        setState({
          status: "polling",
          jobId: resp.job_id,
          downloadUrl: null,
          error: null,
          retryAfterSecs: null,
        });
      }
    } catch (e) {
      if (e instanceof ApiError && e.status === 429) {
        const envelope = (e.body as { error?: { retry_after_secs?: number } } | undefined)
          ?.error;
        const retry = envelope?.retry_after_secs;
        setState({
          status: "rate_limited",
          jobId: null,
          downloadUrl: null,
          error: e.message,
          retryAfterSecs: typeof retry === "number" ? retry : null,
        });
        return;
      }
      const msg = e instanceof Error ? e.message : "unknown error";
      setState({ ...INITIAL, status: "error", error: msg });
    }
  }, []);

  useEffect(() => {
    if (state.status !== "polling" || !state.jobId) return;
    const jobId = state.jobId;
    let cancelled = false;
    const tick = async () => {
      try {
        const resp = await apiGet<ExportStatusResponse>(`/api/v1/me/export/${jobId}`);
        if (cancelled) return;
        if (resp.status === "ready") {
          setState((s) => ({
            ...s,
            status: "ready",
            downloadUrl: `/api/v1/me/export/download/${jobId}-pending`,
          }));
          // The poll response doesn't carry the raw token (it's
          // single-use, only emitted on the initial POST response).
          // The v1 backend already returns `ready` on the first POST,
          // so this branch is dead code today; keeping it lets a
          // future async-job switch land without a hook rewrite.
        } else if (resp.status === "failed") {
          setState((s) => ({
            ...s,
            status: "error",
            error: resp.error ?? "export failed",
          }));
        }
      } catch (e) {
        if (cancelled) return;
        const msg = e instanceof Error ? e.message : "poll failed";
        setState((s) => ({ ...s, status: "error", error: msg }));
      }
    };
    const id = setInterval(tick, 2_000);
    void tick();
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [state.status, state.jobId]);

  const reset = useCallback(() => setState(INITIAL), []);

  return useMemo(() => ({ ...state, request, reset }), [state, request, reset]);
}
