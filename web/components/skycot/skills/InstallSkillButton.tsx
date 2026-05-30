"use client";

import { cn } from "@/lib/utils";
import {
  installButtonLabel,
  useInstallSkill,
  type InstallPhase,
} from "@/lib/hooks/use-skills";

/**
 * Install affordance for the skills marketplace — the Scene-4 beat.
 *
 * Clicking it runs the real "make this skill available to my workers"
 * flow: a short provenance-verify pause, then a live
 * `POST /api/v1/workers/<id>/skills` per teammate
 * (`installSkillToWorkers` in `lib/api/skills.ts`, gated on the same
 * `isAuthBypassed() && !isDemoLive()` mock contract as every other
 * fetcher). The button text walks through the three on-camera states —
 * "Verifying provenance…" → "Installing skill…" → "Available to N
 * workers" — and surfaces a real error if the gateway rejects the
 * install (e.g. no workers hired yet).
 *
 * `skillId` must be the catalog skill's UUID (the gateway's
 * `SkillRow.id` / `SkillUsage.skill_id`). On the list card that is
 * `skill.id`; on the detail page it is `usage.skill_id`.
 *
 * Used on both `/skills` (per card) and `/skills/<id>` (detail), so each
 * instance owns its own install state machine via `useInstallSkill`.
 */

export interface InstallSkillButtonProps {
  /** Catalog skill UUID to assign to the workspace's workers. */
  skillId: string;
  /**
   * When true (list card), the button stops click propagation so it
   * doesn't trigger the surrounding `<Link>` navigation. Detail page
   * leaves this false.
   */
  stopPropagation?: boolean;
  /** Optional className passthrough on the button. */
  className?: string;
  /** Override the data-testid (defaults to `skill-install-btn`). */
  testId?: string;
}

function phaseModifier(phase: InstallPhase): string {
  switch (phase) {
    case "verifying":
    case "installing":
      return "skill-install-btn--busy";
    case "installed":
      return "skill-install-btn--installed";
    case "error":
      return "skill-install-btn--error";
    default:
      return "";
  }
}

export function InstallSkillButton({
  skillId,
  stopPropagation = false,
  className,
  testId,
}: InstallSkillButtonProps) {
  const { phase, workerCount, errorMessage, install } = useInstallSkill();
  const busy = phase === "verifying" || phase === "installing";
  const label = installButtonLabel(phase, workerCount);

  return (
    <div className="skill-install-row" data-testid="skill-install">
      <button
        type="button"
        className={cn("skill-install-btn", phaseModifier(phase), className)}
        data-testid={testId ?? "skill-install-btn"}
        data-phase={phase}
        // `installed` stays clickable so a retry is possible, but reads
        // as the success terminal state until then.
        disabled={busy}
        aria-busy={busy}
        onClick={(e) => {
          if (stopPropagation) {
            e.preventDefault();
            e.stopPropagation();
          }
          install(skillId);
        }}
      >
        {busy && <span className="skill-install-spinner" aria-hidden="true" />}
        {label}
      </button>
      {phase === "error" && errorMessage && (
        <p className="skill-install-error-msg" data-testid="skill-install-error">
          {errorMessage}
        </p>
      )}
    </div>
  );
}
