"use client";

import { ArrowRight } from "lucide-react";
import { useRouter } from "next/navigation";
import { useCallback, useRef, useState, type KeyboardEvent } from "react";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { buttonVariants } from "@/components/ui/Button";
import { StatusPill, type StatusPillProps } from "@/components/ui/StatusPill";
import { useWorkerActions } from "@/lib/hooks/use-worker-actions";
import { useToast } from "@/lib/ui/use-toast";
import { cn } from "@/lib/utils";
import type { Worker, WorkerStatus } from "@/lib/types/worker";
import {
  TeammateCardContextMenu,
  type TeammateCardContextMenuHandle,
} from "./TeammateCardContextMenu";

/**
 * C2 holographic-showcase teammate card.
 *
 * Locked design: docs/skycot/design-explorations/2026-05-18/workers-page/option-C2-holographic-showcase.html.
 * Gradient banner (purple → pink), weight-900 level number, animated
 * shine sweep on hover, large portrait avatar overlapping banner,
 * trait chips. All test surfaces from the prior flat-list card are
 * preserved (status pill, today/tomorrow blocks, Open profile link,
 * context menu) so the brief is a pure visual reskin.
 */

const STATUS_TONES: Record<WorkerStatus, NonNullable<StatusPillProps["tone"]>> = {
  working: "info",
  waiting: "warning",
  idle: "muted",
  blocked: "error",
  paused: "muted",
};

const STATUS_LABELS: Record<WorkerStatus, string> = {
  working: "Working",
  waiting: "Waiting on you",
  idle: "Idle",
  blocked: "Blocked",
  paused: "Paused",
};

const LONG_PRESS_MS = 500;

// Per-name personality + level data. Matches WorkerPersonalityCard's
// PERSONALITIES map for the workers it covers; unknown names fall
// through to a stable hash so the card never renders an empty banner.
const TRAITS_BY_NAME: Record<string, string[]> = {
  Mariah: ["Patient", "Detail-oriented", "Picture-perfect listings", "Polite"],
  Priya: ["Thorough", "Respectful", "Fast", "Calibrated"],
  Jamal: ["Calm under pressure", "Empathic", "Quick", "Honest"],
  Nori: ["Creative", "On-brand", "Concise", "Curious"],
  Sam: ["Vigilant", "Steady", "Methodical"],
  Rio: ["Polite", "Reliable", "Punctual"],
};

const LEVEL_BY_NAME: Record<string, number> = {
  Mariah: 5,
  Priya: 9,
  Jamal: 12,
  Nori: 15,
  Sam: 7,
  Rio: 4,
};

function hashLevel(name: string): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) | 0;
  return (Math.abs(h) % 18) + 2;
}

function levelFor(worker: Worker): number {
  return LEVEL_BY_NAME[worker.name] ?? hashLevel(worker.name);
}

function traitsFor(worker: Worker): string[] {
  return TRAITS_BY_NAME[worker.name] ?? [];
}

function initialsFor(name: string): string {
  const cleaned = name.trim();
  if (!cleaned) return "??";
  const parts = cleaned.split(/\s+/);
  if (parts.length >= 2 && parts[0] && parts[1]) {
    return (parts[0][0] ?? "").concat(parts[1][0] ?? "").toUpperCase();
  }
  return cleaned.slice(0, 2).toUpperCase();
}

function tagFor(worker: Worker): string {
  const v = worker.role.vertical ?? "";
  if (!v) return "TEAMMATE";
  return v.replace(/[-_]/g, " ").toUpperCase();
}

export interface TeammateCardProps {
  worker: Worker;
  className?: string;
  /** Forwarded to outer Card for tests / parents that need a handle. */
  "data-testid"?: string;
}

export function TeammateCard({
  worker,
  className,
  "data-testid": testId,
}: TeammateCardProps) {
  const router = useRouter();
  const menuRef = useRef<TeammateCardContextMenuHandle>(null);
  const longPressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const actions = useWorkerActions(worker.id);
  const { success: toastSuccess, error: toastError } = useToast();
  const [confirm, setConfirm] = useState<"pause" | "resume" | null>(null);

  const cardTestId = testId ?? `teammate-${worker.id}`;
  // Direct-link to /overview rather than /workers/<id> (which is a
  // server-side redirect to /overview). Saves a redirect flicker on
  // every TeammateCard click — wave-12 audit finding.
  const profileHref = `/workers/${worker.id}/overview`;
  const tone = STATUS_TONES[worker.status];
  const statusLabel = STATUS_LABELS[worker.status];
  const level = levelFor(worker);
  const traits = traitsFor(worker);
  const initials = initialsFor(worker.name);
  const tag = tagFor(worker);

  const openProfile = useCallback(() => {
    router.push(profileHref);
  }, [router, profileHref]);

  const onCardClick = useCallback(() => {
    openProfile();
  }, [openProfile]);

  const onCardKeyDown = useCallback(
    (e: KeyboardEvent<HTMLDivElement>) => {
      if (e.key !== "Enter" && e.key !== " ") return;
      if (e.target !== e.currentTarget) return;
      e.preventDefault();
      openProfile();
    },
    [openProfile],
  );

  const onContextMenu = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      e.preventDefault();
      menuRef.current?.open();
    },
    [],
  );

  const startLongPress = useCallback(() => {
    if (longPressTimer.current) clearTimeout(longPressTimer.current);
    longPressTimer.current = setTimeout(() => {
      menuRef.current?.open();
    }, LONG_PRESS_MS);
  }, []);

  const cancelLongPress = useCallback(() => {
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current);
      longPressTimer.current = null;
    }
  }, []);

  const performAction = useCallback(
    async (kind: "pause" | "resume") => {
      try {
        if (kind === "pause") await actions.pauseWorker(worker.id);
        else await actions.resumeWorker(worker.id);
        toastSuccess(
          kind === "pause"
            ? `${worker.name} paused. Resume from her profile.`
            : `${worker.name} resumed.`,
          { id: `worker-${worker.id}-${kind}` },
        );
      } catch {
        toastError(
          kind === "pause"
            ? `Couldn't pause ${worker.name}. Try again.`
            : `Couldn't resume ${worker.name}. Try again.`,
          { id: `worker-${worker.id}-${kind}-error` },
        );
      }
    },
    [actions, worker.id, worker.name, toastSuccess, toastError],
  );

  const bullets = worker.headline?.bullets ?? [];
  const tomorrow = worker.headline?.tomorrow;

  return (
    <>
      <style jsx>{`
        .holo-card {
          position: relative;
          border-radius: 24px;
          overflow: hidden;
          background: var(--card, white);
          box-shadow:
            0 1px 2px rgba(0, 0, 0, 0.06),
            0 32px 60px -16px rgba(139, 92, 246, 0.3),
            inset 0 0 0 2px rgba(139, 92, 246, 0.12);
          transition:
            transform 0.35s cubic-bezier(0.2, 0, 0, 1),
            box-shadow 0.35s cubic-bezier(0.2, 0, 0, 1);
          cursor: pointer;
          outline: none;
          display: flex;
          flex-direction: column;
        }
        .holo-card:focus-visible {
          box-shadow:
            0 1px 2px rgba(0, 0, 0, 0.06),
            0 32px 60px -16px rgba(139, 92, 246, 0.3),
            inset 0 0 0 2px rgba(139, 92, 246, 0.12),
            0 0 0 3px var(--ring, #8b5cf6);
        }
        .holo-card:hover {
          transform: translateY(-8px) rotate(-1deg) scale(1.015);
          box-shadow:
            0 1px 2px rgba(0, 0, 0, 0.06),
            0 48px 80px -16px rgba(139, 92, 246, 0.45),
            inset 0 0 0 2px rgba(139, 92, 246, 0.25);
        }
        .holo-banner {
          position: relative;
          background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 100%);
          color: white;
          padding: 24px 22px 72px;
          overflow: hidden;
        }
        .holo-banner::before {
          content: "";
          position: absolute;
          inset: 0;
          background:
            radial-gradient(circle at 30% 20%, rgba(255, 255, 255, 0.5), transparent 40%),
            radial-gradient(circle at 80% 80%, rgba(255, 255, 255, 0.25), transparent 50%);
          pointer-events: none;
        }
        .holo-shine {
          position: absolute;
          inset: 0;
          background: linear-gradient(
            115deg,
            transparent 35%,
            rgba(255, 255, 255, 0.6) 50%,
            transparent 65%
          );
          background-size: 200% 200%;
          background-position: 200% 50%;
          transition: background-position 1s cubic-bezier(0.2, 0, 0, 1);
          pointer-events: none;
        }
        .holo-card:hover .holo-shine {
          background-position: -50% 50%;
        }
        @media (prefers-reduced-motion: reduce) {
          .holo-card {
            transition: none;
          }
          .holo-card:hover {
            transform: none;
          }
          .holo-shine {
            display: none;
          }
        }
        .holo-banner-row {
          position: relative;
          z-index: 1;
          display: flex;
          align-items: flex-start;
          justify-content: space-between;
          gap: 12px;
        }
        .holo-banner-tag {
          font-size: 10.5px;
          font-weight: 800;
          text-transform: uppercase;
          letter-spacing: 0.16em;
          padding: 5px 12px;
          border-radius: 999px;
          background: rgba(255, 255, 255, 0.2);
          backdrop-filter: blur(10px);
          -webkit-backdrop-filter: blur(10px);
          white-space: nowrap;
        }
        .holo-banner-level {
          text-align: right;
          line-height: 1;
          min-width: 0;
        }
        .holo-banner-level-lbl {
          font-size: 9.5px;
          font-weight: 700;
          letter-spacing: 0.18em;
          opacity: 0.85;
        }
        .holo-banner-level-n {
          font-size: 56px;
          font-weight: 900;
          line-height: 0.9;
          letter-spacing: -0.04em;
          margin-top: 2px;
          text-shadow: 0 4px 16px rgba(0, 0, 0, 0.18);
        }
        .holo-menu-slot {
          position: absolute;
          top: 14px;
          right: 14px;
          z-index: 3;
          display: inline-flex;
        }
        .holo-portrait {
          position: relative;
          z-index: 2;
          margin: -56px 22px 0;
          display: flex;
          align-items: flex-end;
          height: 100px;
        }
        .holo-avatar {
          width: 92px;
          height: 92px;
          border-radius: 26px;
          background: var(--card, white);
          padding: 5px;
          box-shadow: 0 16px 40px -8px rgba(0, 0, 0, 0.2);
        }
        .holo-avatar-inner {
          width: 100%;
          height: 100%;
          border-radius: 21px;
          background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 100%);
          display: grid;
          place-items: center;
          color: white;
          font-size: 28px;
          font-weight: 800;
          letter-spacing: 0.02em;
        }
        .holo-name-block {
          padding: 14px 24px 0;
        }
        .holo-name {
          margin: 0;
          font-size: 26px;
          font-weight: 800;
          letter-spacing: -0.025em;
          color: var(--foreground, #1e1b4b);
          line-height: 1.1;
        }
        .holo-role {
          margin-top: 4px;
          color: var(--muted-foreground, #71717a);
          font-size: 13px;
          font-weight: 600;
        }
        .holo-status-row {
          margin-top: 12px;
        }
        .holo-traits {
          padding: 16px 24px 0;
        }
        .holo-traits-lbl {
          font-size: 9.5px;
          text-transform: uppercase;
          letter-spacing: 0.18em;
          color: var(--muted-foreground, #71717a);
          font-weight: 800;
          margin-bottom: 8px;
        }
        .holo-trait-chips {
          display: flex;
          flex-wrap: wrap;
          gap: 6px;
        }
        .holo-trait-chip {
          font-size: 11px;
          font-weight: 600;
          padding: 4px 10px;
          border-radius: 999px;
          background: rgba(139, 92, 246, 0.08);
          color: #6d28d9;
          border: 1px solid rgba(139, 92, 246, 0.15);
        }
        .holo-body {
          padding: 18px 24px 0;
          display: flex;
          flex-direction: column;
          gap: 10px;
          flex: 1;
        }
        .holo-block-lbl {
          font-size: 9.5px;
          text-transform: uppercase;
          letter-spacing: 0.18em;
          color: var(--muted-foreground, #71717a);
          font-weight: 800;
          margin-bottom: 4px;
        }
        .holo-bullets {
          list-style: none;
          margin: 0;
          padding: 0;
          display: flex;
          flex-direction: column;
          gap: 4px;
          color: var(--foreground, #1e1b4b);
          font-size: 13px;
          line-height: 1.45;
        }
        .holo-bullet {
          display: flex;
          gap: 8px;
        }
        .holo-bullet::before {
          content: "•";
          color: var(--muted-foreground, #71717a);
        }
        .holo-tomorrow {
          font-size: 12px;
          color: var(--muted-foreground, #71717a);
        }
        .holo-tomorrow-lbl {
          font-weight: 700;
          color: var(--foreground, #1e1b4b);
        }
        .holo-foot {
          margin-top: auto;
          padding: 18px 24px 18px;
        }
      `}</style>

      <div
        role="group"
        aria-label={`${worker.name} — ${statusLabel}`}
        tabIndex={0}
        data-testid={cardTestId}
        onClick={onCardClick}
        onKeyDown={onCardKeyDown}
        onContextMenu={onContextMenu}
        onPointerDown={(e) => {
          if (e.pointerType === "touch") startLongPress();
        }}
        onPointerUp={cancelLongPress}
        onPointerCancel={cancelLongPress}
        onPointerLeave={cancelLongPress}
        className={cn("holo-card", className)}
      >
        <div className="holo-banner">
          <span className="holo-shine" aria-hidden />
          <div className="holo-banner-row">
            <span className="holo-banner-tag" data-testid={`teammate-tag-${worker.id}`}>
              {tag}
            </span>
            <div className="holo-banner-level">
              <div className="holo-banner-level-lbl">LEVEL</div>
              <div
                className="holo-banner-level-n"
                data-testid={`teammate-level-${worker.id}`}
              >
                {level}
              </div>
            </div>
          </div>
          <div className="holo-menu-slot">
            <TeammateCardContextMenu
              ref={menuRef}
              status={worker.status}
              workerName={worker.name}
              disabled={actions.isPending}
              onPauseRequested={() => setConfirm("pause")}
              onResumeRequested={() => setConfirm("resume")}
              data-testid={worker.id}
            />
          </div>
        </div>

        <div className="holo-portrait">
          <div className="holo-avatar" aria-hidden>
            <div className="holo-avatar-inner">{initials}</div>
          </div>
        </div>

        <div className="holo-name-block">
          <h2 className="holo-name">{worker.name}</h2>
          <div className="holo-role">{worker.role.tagline}</div>
          <div className="holo-status-row">
            <StatusPill tone={tone} size="sm" data-testid={`teammate-status-${worker.id}`}>
              {statusLabel}
            </StatusPill>
          </div>
        </div>

        {traits.length > 0 ? (
          <div className="holo-traits" data-testid={`teammate-traits-${worker.id}`}>
            <div className="holo-traits-lbl">Traits</div>
            <div className="holo-trait-chips">
              {traits.map((t) => (
                <span key={t} className="holo-trait-chip">
                  {t}
                </span>
              ))}
            </div>
          </div>
        ) : null}

        {bullets.length > 0 || tomorrow ? (
          <div className="holo-body">
            {bullets.length > 0 ? (
              <div data-testid={`teammate-today-${worker.id}`}>
                <div className="holo-block-lbl">Today</div>
                <ul className="holo-bullets">
                  {bullets.slice(0, 3).map((b, i) => (
                    <li key={`${worker.id}-bullet-${i}`} className="holo-bullet">
                      <span className="min-w-0 flex-1">{b}</span>
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}
            {tomorrow ? (
              <div data-testid={`teammate-tomorrow-${worker.id}`} className="holo-tomorrow">
                <span className="holo-tomorrow-lbl">Tomorrow:</span> {tomorrow}
              </div>
            ) : null}
          </div>
        ) : null}

        <div className="holo-foot">
          <a
            href={profileHref}
            data-testid={`teammate-open-profile-${worker.id}`}
            onClick={(e) => {
              e.stopPropagation();
              e.preventDefault();
              openProfile();
            }}
            className={cn(
              buttonVariants({ variant: "ghost", size: "sm" }),
              "w-full justify-center text-[var(--muted-foreground)] hover:text-[var(--foreground)]",
            )}
          >
            Open profile
            <ArrowRight aria-hidden className="size-3.5" />
          </a>
        </div>
      </div>

      <AlertDialog
        open={confirm !== null}
        onOpenChange={(next) => {
          if (!next) setConfirm(null);
        }}
        title={
          confirm === "resume"
            ? `Resume ${worker.name}?`
            : `Pause ${worker.name}?`
        }
        description={
          confirm === "resume"
            ? "They'll pick up where they left off."
            : "They'll stop taking new actions until you resume them. Current pending approvals stay where they are."
        }
        confirmLabel={confirm === "resume" ? "Resume" : "Pause"}
        cancelLabel="Cancel"
        confirmVariant={confirm === "resume" ? "primary" : "secondary"}
        onConfirm={() => {
          const kind = confirm;
          setConfirm(null);
          if (kind) void performAction(kind);
        }}
      />
    </>
  );
}
