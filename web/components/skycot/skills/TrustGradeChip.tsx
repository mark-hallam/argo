"use client";

import { cn } from "@/lib/utils";

/**
 * Trust-grade chip for the skill-detail page.
 *
 * Sources the grade from the canonical `argo_skill::TrustGrade` enum
 * (#1949). The five rungs map to a four-letter ladder so the chip
 * can carry the trust signal at a glance — letter grades are a
 * deliberate UI deviation from the underlying snake_case discriminants
 * (which surface verbatim in the gateway response shape). The tone
 * column matches the canonical mapping in
 * `crates/argo-cli/src/handlers/gateway/routes_worker_trust_grade.rs`
 * so the chip reads the same colour as the worker trust-grade badge
 * across the SPA.
 *
 * | TrustGrade discriminant | Letter | Tone        |
 * |-------------------------|--------|-------------|
 * | `builtin`               | A      | success     |
 * | `published_verified`    | A      | success     |
 * | `tenant`                | B      | warning     |
 * | `published_community`   | C      | warning     |
 * | `learned_unreviewed`    | D      | destructive |
 * | (null / unknown)        | —      | muted       |
 *
 * The chip renders "Ungraded" in muted gray when no trust data is
 * available for a skill (e.g. a learned skill that has not yet been
 * graded). The label "Trust · <letter>" pairs the rung with the
 * fully-spelled discriminant under the chip's `data-grade` attribute
 * for downstream tests and audit-trail readers.
 */

export interface TrustGradeChipProps {
  /** Canonical `argo_skill::TrustGrade` discriminant from the gateway,
   *  or `null`/`undefined` if the skill has no trust data yet. */
  grade: string | null | undefined;
  /** Optional className passthrough. */
  className?: string;
  /** Override the data-testid (defaults to `skill-trust-grade`). */
  testId?: string;
}

type ChipTone = "success" | "warning" | "destructive" | "muted";

interface ChipMapping {
  letter: string;
  tone: ChipTone;
  /** Human-friendly tooltip / sr-only context. */
  description: string;
}

const UNGRADED: ChipMapping = {
  letter: "—",
  tone: "muted",
  description: "Ungraded — no trust signal recorded yet",
};

const GRADE_TABLE: Record<string, ChipMapping> = {
  builtin: { letter: "A", tone: "success", description: "Built-in — ships in the binary" },
  published_verified: {
    letter: "A",
    tone: "success",
    description: "Verified publisher — signed marketplace skill",
  },
  tenant: { letter: "B", tone: "warning", description: "Workspace — your organization's skill" },
  published_community: {
    letter: "C",
    tone: "warning",
    description: "Community — published, not verified",
  },
  learned_unreviewed: {
    letter: "D",
    tone: "destructive",
    description: "Learned — not yet reviewed",
  },
};

function lookupGrade(grade: string | null | undefined): ChipMapping | null {
  if (typeof grade !== "string" || grade.length === 0) return null;
  return GRADE_TABLE[grade] ?? null;
}

export function TrustGradeChip({ grade, className, testId }: TrustGradeChipProps) {
  const mapping = lookupGrade(grade);
  const isUngraded = mapping === null;
  const resolved = mapping ?? UNGRADED;
  const label = isUngraded ? "Ungraded" : `Trust · ${resolved.letter}`;

  return (
    <span
      className={cn("trust-grade-chip", `trust-grade-chip--${resolved.tone}`, className)}
      data-testid={testId ?? "skill-trust-grade"}
      data-grade={isUngraded ? "ungraded" : (grade as string)}
      data-tone={resolved.tone}
      data-letter={resolved.letter}
      title={resolved.description}
      aria-label={resolved.description}
    >
      {label}
    </span>
  );
}
