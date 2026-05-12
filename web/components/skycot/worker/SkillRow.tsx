"use client";

import { type ReactNode } from "react";
import { Sparkles } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  SKILL_SOURCE_LABELS,
  trustTone,
  type SkillSource,
  type TrustTone,
} from "@/lib/api/worker-skills";

/**
 * Shared row primitive for the Skills tab. Renders the icon + name +
 * version + trust badge + a meta string (e.g. "used 128× last 7d" for
 * Active, "Workspace" source for Library, "8 ✓ · 0 ✗" for Learned),
 * with the per-section actions slotted at the end.
 *
 * The three section cards build the row by passing only the data the
 * section needs; the layout (alignment, spacing, focus-ring) is
 * locked here so all three look identical.
 */

const TRUST_TONE_CLASS: Record<TrustTone, string> = {
  success: "bg-[var(--success)]/15 text-[var(--success)]",
  warning: "bg-[var(--warning)]/15 text-[var(--warning)]",
  destructive: "bg-[var(--destructive)]/15 text-[var(--destructive)]",
};

export interface TrustBadgeProps {
  trustScore: number;
  className?: string;
}

export function TrustBadge({ trustScore, className }: TrustBadgeProps) {
  const tone = trustTone(trustScore);
  return (
    <span
      data-testid="trust-badge"
      data-tone={tone}
      className={cn(
        "inline-flex shrink-0 items-center rounded-full px-2 py-0.5 text-xs font-medium",
        TRUST_TONE_CLASS[tone],
        className,
      )}
    >
      trust: {trustScore}%
    </span>
  );
}

export interface SourceBadgeProps {
  source: SkillSource;
}

export function SourceBadge({ source }: SourceBadgeProps) {
  return (
    <span
      data-testid={`skill-source-${source}`}
      className="shrink-0 rounded-full bg-[var(--muted)] px-2 py-0.5 text-xs text-[var(--muted-foreground)]"
    >
      {SKILL_SOURCE_LABELS[source]}
    </span>
  );
}

export interface SkillRowProps {
  /** Stable id used to derive `data-testid="skill-row-<id>"`. */
  id: string;
  /** Display label — skill name + version, or learned-pattern label. */
  title: string;
  /** Optional pre-formatted version suffix to render after the title. */
  version?: string;
  /** Right-of-title pills: trust badge + source / usage / counts. */
  meta?: ReactNode;
  /** Right-aligned action buttons. */
  actions: ReactNode;
  /** Optional aria-label override for the row container. */
  ariaLabel?: string;
}

export function SkillRow({ id, title, version, meta, actions, ariaLabel }: SkillRowProps) {
  return (
    <li
      data-testid={`skill-row-${id}`}
      aria-label={ariaLabel ?? title}
      className="flex flex-wrap items-center gap-x-3 gap-y-2 py-3"
    >
      <Sparkles
        aria-hidden
        className="size-4 shrink-0 text-[var(--muted-foreground)]"
      />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-[var(--foreground)]">
          <span data-testid={`skill-row-${id}-title`}>{title}</span>
          {version ? (
            <span className="ml-1 text-xs font-normal text-[var(--muted-foreground)]">
              {version}
            </span>
          ) : null}
        </p>
        {meta ? (
          <div className="mt-1 flex flex-wrap items-center gap-2">{meta}</div>
        ) : null}
      </div>
      <div className="flex flex-wrap items-center gap-1.5">{actions}</div>
    </li>
  );
}
