"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { AlertTriangle, RotateCcw } from "lucide-react";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import { HeroInput } from "@/components/skycot/genesis/HeroInput";
import { ProgressStrip } from "@/components/skycot/genesis/ProgressStrip";
import { RoleContractReview } from "@/components/skycot/genesis/RoleContractReview";
import {
  SopUpload,
  type SopSelection,
} from "@/components/skycot/genesis/SopUpload";
import { DocumentUploader } from "@/components/skycot/genesis/DocumentUploader";
import {
  CadenceExceptionsCapture,
  EMPTY_CADENCE_EXCEPTIONS,
  type CadenceExceptionsSelection,
} from "@/components/skycot/genesis/CadenceExceptionsCapture";
import {
  GenesisModeToggle,
  type GenesisMode,
} from "@/components/skycot/genesis/GenesisModeToggle";
import {
  ConversationIntake,
  type ConversationGatheredSlots,
} from "@/components/skycot/genesis/ConversationIntake";
import { BoundariesPanel, EMPTY_BOUNDARIES, type BoundariesSpec } from "@/components/skycot/genesis/BoundariesPanel";
import {
  GENESIS_PHASES,
  hireFromGenesis,
  streamGenesis,
  type GenesisErrorEvent,
  type GenesisPhase,
  type GenesisVerticalReadyEvent,
  type GenesisStartParams,
  type SuccessCriterion,
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
  // Genesis P6 — the moat (DEC-025 #5): an optional SOP the teammate must
  // comprehend before hire. `null` until the operator attaches one.
  const [sop, setSop] = useState<SopSelection | null>(null);
  // UF Phase-A · A1-wire: the operator's intake-capture slots (WHEN it runs,
  // WHAT carve-outs apply, HOW success is measured). Threaded into the SSE
  // request so they land on the teammate's `CapabilitySpec` (and surface back
  // on the `vertical_ready` review), not dropped on the floor.
  const [intake, setIntake] = useState<CadenceExceptionsSelection>(
    EMPTY_CADENCE_EXCEPTIONS,
  );
  // UF Phase-A · A3 (DEC-039): "Describe it" (one-shot) ↔ "Talk it through"
  // (conversational intake). ADDITIVE — both land on the identical review →
  // hire path below; the toggle only chooses how the role is described.
  const [mode, setMode] = useState<GenesisMode>("describe");
  // Phase-2 access-controls + guardrails: the operator's "Set boundaries" spec,
  // threaded into the SSE request and synthesized into an enforceable
  // truth-boundary policy on the gateway.
  const [boundaries, setBoundaries] = useState<BoundariesSpec>(EMPTY_BOUNDARIES);
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

  // Shared one-shot SSE kickoff. Both "Describe it" (handleSubmit) and "Talk it
  // through" (handleConversationComplete) funnel through here with EXPLICIT
  // params so the conversational path reaches the SAME materialize → review →
  // hire flow as the textarea — no parallel client-side materializer (DEC-039).
  const startStream = useCallback(
    (params: GenesisStartParams) => {
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
          roleDescription: params.roleDescription,
          tenantId: tenantIdRef.current,
          overwrite: params.isRetry,
          // P6: attach the SOP (if any) so the teammate is ingested + quizzed
          // against the SPECIFIC document before it can be hired.
          ...(params.sopDocument
            ? {
                sopDocument: params.sopDocument,
                sopDocumentFormat: params.sopDocumentFormat,
                sopFilename: params.sopFilename,
              }
            : {}),
          // A1-wire: forward the intake-capture slots (each omitted from the
          // wire when blank, so the text-only path is unchanged).
          ...(params.cadence && params.cadence.trim().length > 0
            ? { cadence: params.cadence }
            : {}),
          ...(params.exceptions && params.exceptions.length > 0
            ? { exceptions: params.exceptions }
            : {}),
          ...(params.successCriteria && params.successCriteria.length > 0
            ? { successCriteria: params.successCriteria }
            : {}),
          ...(params.accessIntegrations?.length ? { accessIntegrations: params.accessIntegrations } : {}),
          ...(typeof params.escalationFailuresThreshold === "number"
            ? { escalationFailuresThreshold: params.escalationFailuresThreshold }
            : {}),
          ...(params.escalationTo ? { escalationTo: params.escalationTo } : {}),
          ...(params.boundaries ? { boundaries: params.boundaries } : {}),
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
        // Don't overwrite an error event already set by the SSE `onError`
        // callback (e.g. the gateway's specific "output dir is not empty"
        // reason). The stream-close .catch fires AFTER onError on the
        // gateway's abrupt half-close, and would otherwise replace the
        // specific message with chrome's generic "network error" text.
        // See fix-spa-genesis-catch-overwrites-sse-error-event-message.
        setErrorEvent((prev) =>
          prev ?? {
            phase: "drafting_persona",
            message:
              err instanceof Error
                ? err.message
                : "Couldn't reach the Genesis stream.",
            recoverable: true,
          },
        );
        setStatus("error");
      });
    },
    [],
  );

  const handleSubmit = useCallback(() => {
    if (status === "streaming" || status === "hiring") return;
    // The "Try again" button re-invokes this from the error state. On
    // that path the backend's per-tenant+persona output dir already
    // exists from the first attempt, so we forward `overwrite: true`
    // to wipe it. Fresh submissions ("idle"/"ready") leave the flag
    // off so the default refuse-to-clobber safety still applies if
    // the operator submits the same description twice by accident.
    const isRetry = status === "error";
    startStream({
      roleDescription,
      cadence: intake.cadence,
      exceptions: intake.exceptions,
      successCriteria: intake.success_criteria,
      ...(sop
        ? {
            sopDocument: sop.document,
            sopDocumentFormat: sop.format,
            sopFilename: sop.filename,
          }
        : {}),
      boundaries,
      isRetry,
    });
  }, [roleDescription, sop, intake, boundaries, status, startStream]);

  // "Talk it through" confirmed: hand the gathered slots to the SAME one-shot
  // stream. We also mirror them into the describe-mode state so a downstream
  // "Try again" / "Modify description" stays coherent.
  const handleConversationComplete = useCallback(
    (gathered: ConversationGatheredSlots) => {
      setRoleDescription(gathered.roleDescription);
      setIntake({
        cadence: gathered.cadence,
        exceptions: gathered.exceptions,
        success_criteria: gathered.successCriteria,
      });
      startStream({
        roleDescription: gathered.roleDescription,
        cadence: gathered.cadence,
        exceptions: gathered.exceptions,
        successCriteria: gathered.successCriteria,
        accessIntegrations: gathered.accessIntegrations,
        escalationFailuresThreshold: gathered.escalationFailuresThreshold,
        escalationTo: gathered.escalationTo,
        boundaries,
        isRetry: false,
      });
    },
    [boundaries, startStream],
  );

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
          background: var(--background);
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
            eyebrowLabel="Genesis · onboarding studio"
            pulseDot
            title="Onboard a new teammate."
            titleAccent="teammate"
            sub="Describe the role, then optionally attach one SOP or playbook. Genesis builds the teammate and checks it comprehends your document before it can start work."
            subMaxWidth={560}
          />
          {status === "idle" ? (
            <GenesisModeToggle mode={mode} onChange={setMode} disabled={isBusy} />
          ) : null}
          {/* "Describe it" (one-shot) — the original, unchanged flow. Always the
              path on the error state so "Try again" / "Modify description"
              behave identically regardless of how the role was first described. */}
          {mode === "describe" || status === "error" ? (
            <>
              <HeroInput
                value={roleDescription}
                onChange={setRoleDescription}
                onSubmit={handleSubmit}
                isBusy={isBusy}
              />
              {status === "idle" || status === "error" ? (
                <>
                  <SopUpload value={sop} onChange={setSop} disabled={isBusy} />
                  <CadenceExceptionsCapture
                    value={intake}
                    onChange={setIntake}
                    disabled={isBusy}
                  />
                  <BoundariesPanel
                    value={boundaries}
                    onChange={setBoundaries}
                    disabled={isBusy}
                  />
                </>
              ) : null}
            </>
          ) : null}
          {/* "Talk it through" — additive conversational intake (DEC-039). On
              confirm it hands the gathered slots to the SAME stream above. */}
          {mode === "talk" && status === "idle" ? (
            <ConversationIntake
              tenantId={tenantIdRef.current}
              onComplete={handleConversationComplete}
              disabled={isBusy}
            />
          ) : null}
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
              {/* Phase 2 — drop SOPs / wikis / process docs so the teammate
                  boots already knowing the material, before confirming hire. */}
              <DocumentUploader
                verticalId={vertical.vertical_id}
                disabled={status === "hiring"}
              />
              {vertical.boundaries ? (
                <BoundariesPanel applied={vertical.boundaries} />
              ) : null}
              <RoleContractReview
                ready={vertical}
                onConfirmHire={handleHire}
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
