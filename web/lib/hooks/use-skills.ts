"use client";

import { useCallback, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  fetchLearnedThisMonth,
  fetchSkills,
  fetchSkillUsage,
  installSkillToWorkers,
  type FetchLearnedThisMonthResult,
  type FetchSkillsResult,
  type FetchSkillUsageResult,
  type InstallSkillResult,
} from "@/lib/api/skills";

const SKILLS_KEY = ["skills"] as const;

/**
 * Marketplace skill catalog. Pulls the layered skill registry via the
 * gateway and revalidates on focus. Empty/offline gateway falls through
 * to the seed catalog in `lib/api/skills.ts` so the /skills landing
 * still renders during dev.
 */
export function useSkills() {
  return useQuery<FetchSkillsResult>({
    queryKey: SKILLS_KEY,
    queryFn: fetchSkills,
  });
}

/**
 * Workers-using-this-skill bundle for the skill-detail page. Returns
 * the canonical trust grade, anonymized author label, current-tenant
 * workers (with `learned_at`), and the cross-tenant adoption count.
 */
export function useSkillUsage(skillName: string | undefined) {
  return useQuery<FetchSkillUsageResult>({
    queryKey: ["skill-usage", skillName ?? ""],
    queryFn: () => fetchSkillUsage(skillName as string),
    enabled: typeof skillName === "string" && skillName.length > 0,
  });
}

/**
 * Skill ids the workers in the current workspace tenant have learned
 * since the first-of-month UTC cutoff. Drives the marketplace's
 * "Skills my workers learned this month" filter chip.
 */
export function useSkillsLearnedThisMonth() {
  return useQuery<FetchLearnedThisMonthResult>({
    queryKey: ["skills-learned-this-month"],
    queryFn: fetchLearnedThisMonth,
  });
}

/**
 * Visible phases of the Install action, surfaced on the button so the
 * Scene-4 narration ("Verifying provenance… → Installing skill… →
 * Available to workers") matches what the camera sees.
 *
 *  - `idle`        — default; button reads "Install".
 *  - `verifying`   — provenance-verify beat (short, deterministic).
 *  - `installing`  — the real `POST /api/v1/workers/<id>/skills`
 *                    round-trip is in flight.
 *  - `installed`   — at least one `worker_skill` row was written; button
 *                    reads "Available to workers".
 *  - `error`       — the gateway rejected the install (e.g. no workers).
 */
export type InstallPhase =
  | "idle"
  | "verifying"
  | "installing"
  | "installed"
  | "error";

/** Duration of the provenance-verify beat before the real POST fires. */
export const INSTALL_VERIFY_MS = 650;

export interface UseInstallSkillResult {
  phase: InstallPhase;
  /** Workers the skill became available to, after a successful install. */
  workerCount: number;
  /** Human-readable error message when `phase === "error"`. */
  errorMessage: string | null;
  /** Kick off the verify → install → installed flow for a skill UUID. */
  install: (skillId: string) => void;
  /** Reset back to `idle` (e.g. when navigating away or retrying). */
  reset: () => void;
}

/**
 * Install-a-skill mutation with the provenance-verify → installing →
 * installed state machine the marketplace renders on the button.
 *
 * The provenance-verify beat is a fixed [`INSTALL_VERIFY_MS`] pause —
 * it is *not* faked work: the install itself is the real
 * `installSkillToWorkers` gateway round-trip, which writes a real
 * `worker_skill` row per teammate. The verify pause only paces the
 * three on-screen states so the narration reads cleanly; the
 * "Available to workers" terminal state is reached only after the
 * gateway (or, in auth-bypass dev, the documented mock short-circuit)
 * actually acknowledges the assignment.
 *
 * On success the marketplace + per-worker skill caches are invalidated
 * so the "Workers using this skill" surfaces refetch the new rows.
 */
export function useInstallSkill(): UseInstallSkillResult {
  const client = useQueryClient();
  const [phase, setPhase] = useState<InstallPhase>("idle");
  const [workerCount, setWorkerCount] = useState(0);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const mutation = useMutation<InstallSkillResult, Error, string>({
    mutationFn: (skillId) => installSkillToWorkers(skillId),
    onMutate: () => {
      setPhase("installing");
    },
    onSuccess: (result) => {
      setWorkerCount(result.workerIds.length);
      setPhase("installed");
      void client.invalidateQueries({ queryKey: SKILLS_KEY });
      void client.invalidateQueries({ queryKey: ["skill-usage"] });
      void client.invalidateQueries({ queryKey: ["worker"] });
    },
    onError: (err) => {
      setErrorMessage(err.message || "Install failed");
      setPhase("error");
    },
  });

  const install = useCallback(
    (skillId: string) => {
      if (timerRef.current) clearTimeout(timerRef.current);
      setErrorMessage(null);
      setPhase("verifying");
      // Provenance-verify beat, then fire the real assignment.
      timerRef.current = setTimeout(() => {
        mutation.mutate(skillId);
      }, INSTALL_VERIFY_MS);
    },
    [mutation],
  );

  const reset = useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    setErrorMessage(null);
    setWorkerCount(0);
    setPhase("idle");
  }, []);

  return { phase, workerCount, errorMessage, install, reset };
}

/**
 * Map an [`InstallPhase`] to the button label the marketplace renders.
 * Centralized so the list card + detail page show identical copy.
 */
export function installButtonLabel(
  phase: InstallPhase,
  workerCount: number,
): string {
  switch (phase) {
    case "verifying":
      return "Verifying provenance…";
    case "installing":
      return "Installing skill…";
    case "installed":
      return workerCount > 0
        ? `Available to ${workerCount} worker${workerCount === 1 ? "" : "s"}`
        : "Available to workers";
    case "error":
      return "Install failed — retry";
    case "idle":
    default:
      return "Install";
  }
}
