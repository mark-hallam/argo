"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { AlertTriangle, RotateCcw } from "lucide-react";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import { HeroInput } from "@/components/skycot/genesis/HeroInput";
import { ProgressStrip } from "@/components/skycot/genesis/ProgressStrip";
import { PreviewCard } from "@/components/skycot/genesis/PreviewCard";
import {
  GENESIS_PHASES,
  hireFromGenesis,
  streamGenesis,
  type GenesisErrorEvent,
  type GenesisPhase,
  type GenesisVerticalReadyEvent,
} from "@/lib/api/genesis";

/**
 * `/genesis` — turn plain English into a real AI teammate.
 *
 * Calls `POST /api/v1/genesis` (Server-Sent Events) live, renders a
 * brand-loud progress strip + preview card as the stream advances,
 * then commits the worker via `POST /api/v1/workers` and navigates to
 * `/workers/{id}`. No mock fallback — per `feedback_no_theater_in_demo`
 * the demo path is the real backend pipeline.
 *
 * The tenant id (forwarded as `X-Argo-Tenant-Id` on BOTH calls) is a
 * stable per-session slug so the hire-side vertical-resolution can find
 * the freshly-generated `.genesis/<tenant>/<vertical>/`.
 */

const TENANT_STORAGE_KEY = "skycot.genesisTenantId";

function generateTenantSlug(): string {
  const rand =
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
  // 24 hex chars is plenty for a per-browser id, keeps the audit path
  // legible.
  return `op-${rand.replace(/-/g, "").slice(0, 18)}`;
}

function getOrCreateTenantId(): string {
  if (typeof window === "undefined") return "anonymous";
  try {
    const existing = window.localStorage.getItem(TENANT_STORAGE_KEY);
    if (existing && existing.trim().length > 0) return existing;
    const next = generateTenantSlug();
    window.localStorage.setItem(TENANT_STORAGE_KEY, next);
    return next;
  } catch {
    return generateTenantSlug();
  }
}

type Status = "idle" | "streaming" | "ready" | "hiring" | "error";

export default function GenesisPage() {
  const router = useRouter();
  const [roleDescription, setRoleDescription] = useState("");
  const [status, setStatus] = useState<Status>("idle");
  const [activePhase, setActivePhase] = useState<GenesisPhase | null>(null);
  const [vertical, setVertical] = useState<GenesisVerticalReadyEvent | null>(null);
  const [errorEvent, setErrorEvent] = useState<GenesisErrorEvent | null>(null);
  const [hireError, setHireError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  // Lazily read the per-browser tenant id once on mount so the first
  // server-render pass doesn't try to touch `window.localStorage`.
  const tenantIdRef = useRef<string>("anonymous");
  useEffect(() => {
    tenantIdRef.current = getOrCreateTenantId();
  }, []);

  useEffect(
    () => () => {
      abortRef.current?.abort();
    },
    [],
  );

  const resetAttempt = useCallback(() => {
    setStatus("idle");
    setActivePhase(null);
    setVertical(null);
    setErrorEvent(null);
    setHireError(null);
  }, []);

  const handleSubmit = useCallback(() => {
    if (status === "streaming" || status === "hiring") return;
    setStatus("streaming");
    setActivePhase(null);
    setVertical(null);
    setErrorEvent(null);
    setHireError(null);
    const controller = new AbortController();
    abortRef.current?.abort();
    abortRef.current = controller;
    streamGenesis(
      {
        roleDescription,
        tenantId: tenantIdRef.current,
        signal: controller.signal,
      },
      {
        onProgress: (ev) => {
          // Late events for earlier phases shouldn't rewind the strip.
          setActivePhase((prev) => {
            if (prev === null) return ev.phase;
            const prevIdx = GENESIS_PHASES.indexOf(prev);
            const nextIdx = GENESIS_PHASES.indexOf(ev.phase);
            return nextIdx > prevIdx ? ev.phase : prev;
          });
        },
        onVerticalReady: (ev) => {
          setVertical(ev);
          setActivePhase("ready");
          setStatus("ready");
        },
        onError: (ev) => {
          setErrorEvent(ev);
          setStatus("error");
        },
      },
    ).catch((err) => {
      if (controller.signal.aborted) return;
      setErrorEvent({
        phase: activePhase ?? "drafting_persona",
        message:
          err instanceof Error
            ? err.message
            : "Couldn't reach the Genesis stream.",
        recoverable: true,
      });
      setStatus("error");
    });
  }, [roleDescription, status, activePhase]);

  const handleHire = useCallback(async () => {
    if (!vertical) return;
    setStatus("hiring");
    setHireError(null);
    try {
      const result = await hireFromGenesis({
        verticalReady: vertical,
        roleDescription,
        tenantId: tenantIdRef.current,
      });
      router.push(`/workers/${encodeURIComponent(result.workerId)}`);
    } catch (err) {
      const message =
        err instanceof Error && err.message
          ? err.message
          : "Hire failed. Please retry.";
      setHireError(message);
      setStatus("ready");
    }
  }, [vertical, roleDescription, router]);

  const isBusy = status === "streaming";

  return (
    <>
      <style jsx global>{`
        .gen-page-bg {
          position: relative;
          min-height: calc(100vh - 64px);
          background: #fafaff;
          overflow: hidden;
        }
        .gen-page-bg::before {
          content: "";
          position: absolute;
          top: -180px;
          right: -120px;
          width: 540px;
          height: 540px;
          background: radial-gradient(
            circle,
            rgba(139, 92, 246, 0.18) 0%,
            transparent 70%
          );
          pointer-events: none;
        }
        .gen-page-bg::after {
          content: "";
          position: absolute;
          bottom: -200px;
          left: -120px;
          width: 600px;
          height: 600px;
          background: radial-gradient(
            circle,
            rgba(236, 72, 153, 0.14) 0%,
            transparent 70%
          );
          pointer-events: none;
        }
        .gen-page-content {
          position: relative;
          z-index: 1;
          padding-bottom: 72px;
        }
        .gen-error-panel {
          max-width: 880px;
          margin: 32px auto 0;
          padding: 18px 22px;
          border: 1px solid #fecaca;
          background: rgba(254, 226, 226, 0.55);
          border-radius: 14px;
          display: flex;
          align-items: flex-start;
          gap: 14px;
          color: #7f1d1d;
        }
        .gen-error-actions {
          display: flex;
          gap: 10px;
          margin-top: 12px;
        }
        .gen-error-btn {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 8px 14px;
          border-radius: 10px;
          font-family: "Inter", sans-serif;
          font-size: 13px;
          font-weight: 600;
          border: 1px solid #fca5a5;
          background: rgba(255, 255, 255, 0.85);
          color: #7f1d1d;
          cursor: pointer;
        }
        .gen-error-btn.is-primary {
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          border: none;
          color: white;
        }
      `}</style>
      <div className="gen-page-bg" data-testid="genesis-page">
        <div className="gen-page-content">
          <BrandLoudHero
            eyebrowDotColor="#a7f3d0"
            eyebrowLabel="Genesis — describe a role"
            pulseDot
            title="Hire your next teammate."
            titleAccent="teammate"
            sub="Tell us what they should own. We'll draft a real worker — persona, charter, and skills — in about ten seconds. No templates."
            subMaxWidth={520}
          />
          <HeroInput
            value={roleDescription}
            onChange={setRoleDescription}
            onSubmit={handleSubmit}
            isBusy={isBusy}
          />
          {(status === "streaming" ||
            status === "ready" ||
            status === "hiring" ||
            (status === "error" && activePhase !== null)) && (
            <ProgressStrip
              activePhase={activePhase}
              hasError={status === "error"}
            />
          )}
          {status === "error" && errorEvent ? (
            <div className="gen-error-panel" role="alert" data-testid="genesis-error">
              <AlertTriangle size={20} aria-hidden style={{ marginTop: 2 }} />
              <div>
                <strong>{errorEvent.message}</strong>
                <div className="gen-error-actions">
                  <button
                    type="button"
                    className="gen-error-btn is-primary"
                    onClick={handleSubmit}
                    data-testid="genesis-error-retry"
                  >
                    <RotateCcw size={14} aria-hidden /> Try again
                  </button>
                  <button
                    type="button"
                    className="gen-error-btn"
                    onClick={resetAttempt}
                    data-testid="genesis-error-modify"
                  >
                    Modify description
                  </button>
                </div>
              </div>
            </div>
          ) : null}
          {vertical && (status === "ready" || status === "hiring") ? (
            <>
              <PreviewCard
                ready={vertical}
                roleDescription={roleDescription}
                onHire={handleHire}
                isHiring={status === "hiring"}
              />
              {hireError ? (
                <div
                  className="gen-error-panel"
                  role="alert"
                  data-testid="genesis-hire-error"
                  style={{ marginTop: 18 }}
                >
                  <AlertTriangle size={20} aria-hidden style={{ marginTop: 2 }} />
                  <div>
                    <strong>{hireError}</strong>
                  </div>
                </div>
              ) : null}
            </>
          ) : null}
        </div>
      </div>
    </>
  );
}
