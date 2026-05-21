"use client";
import "./page.css";

import Link from "next/link";
import { use } from "react";
import { useSkillUsage } from "@/lib/hooks/use-skills";
import { formatLearnedAgo } from "@/lib/format-learned-ago";

/**
 * Skill detail surface — mission-skill-discovery-marketplace-surface.
 *
 * Wires the existing skill-library substrate (#1682) and trust-engine
 * canonical types (#1949) to the locked A1 marketplace register so
 * mission claim 4 is *visibly* real, not just substrate-real. The
 * page surfaces:
 *
 *   - Authorship label (anonymized — "Authored by SKYCOT" or "your org").
 *   - Trust grade chip sourced from the canonical TrustGrade enum.
 *   - Workers using this skill in the current tenant, each with a
 *     "Learned N weeks ago" learning-loop chip.
 *   - Cross-tenant adoption count (aggregate integer only — no
 *     per-tenant identities leak).
 *
 * Plain-language copy per `user_plain_language_no_jargon.md`: "Used by"
 * not "Adopted by"; "Learned" not "Acquired".
 */

const TRUST_GRADE_LABELS: Record<string, string> = {
  builtin: "Built-in",
  published_verified: "Verified",
  tenant: "Workspace",
  published_community: "Community",
  learned_unreviewed: "Learned (unreviewed)",
};

function trustGradeLabel(grade: string): string {
  return TRUST_GRADE_LABELS[grade] ?? grade;
}

export default function SkillDetailStubPage({
  params,
}: {
  params: Promise<{ skillId: string }>;
}) {
  const { skillId } = use(params);
  const usageQuery = useSkillUsage(skillId);
  const usage = usageQuery.data?.usage;

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600&display=swap"
        rel="stylesheet"
      />
      <div className="stub-stage">
        <div className="stub-eyebrow">SKILL DETAIL</div>
        <h1 className="stub-title">
          <strong>{skillId}</strong>
        </h1>
        <p className="stub-sub">
          Where this skill came from, which of your workers are using it, and how widely
          it&apos;s been picked up across the marketplace.
        </p>

        <section
          className="usage-card"
          aria-labelledby="usage-provenance-heading"
          data-testid="skill-provenance"
        >
          <h2 className="usage-card-title" id="usage-provenance-heading">
            Provenance
          </h2>
          {usageQuery.isPending ? (
            <p className="usage-empty">Loading provenance…</p>
          ) : (
            <div className="usage-chip-row">
              <span
                className="usage-chip usage-chip-author"
                data-testid="skill-author-label"
              >
                {usage?.author_label ?? "Authored by your org"}
              </span>
              <span
                className="usage-chip usage-chip-trust"
                data-testid="skill-trust-grade"
              >
                Trust · {trustGradeLabel(usage?.trust_grade ?? "tenant")}
              </span>
            </div>
          )}
        </section>

        <section
          className="usage-card"
          aria-labelledby="usage-workers-heading"
          data-testid="skill-workers-using"
        >
          <h2 className="usage-card-title" id="usage-workers-heading">
            Workers using this skill
          </h2>
          {usageQuery.isPending ? (
            <p className="usage-empty">Loading workers…</p>
          ) : usage && usage.workers_using.length > 0 ? (
            <ul className="usage-list" role="list">
              {usage.workers_using.map((w) => (
                <li
                  className="usage-row"
                  key={w.worker_id}
                  data-testid={`skill-worker-${w.worker_id}`}
                >
                  <span className="usage-row-name">{w.name}</span>
                  <span
                    className="usage-row-learned"
                    data-testid={`skill-worker-learned-${w.worker_id}`}
                  >
                    {formatLearnedAgo(w.learned_at) ?? "Learned recently"}
                  </span>
                </li>
              ))}
            </ul>
          ) : (
            <p className="usage-empty" data-testid="skill-workers-empty">
              No workers are using this skill yet. Hire a worker and add this skill from
              their Skills tab to put it to work.
            </p>
          )}
          <p className="usage-adoption" data-testid="skill-adoption-count">
            Used by <strong>{usage?.adoption_count ?? 0}</strong> worker
            {(usage?.adoption_count ?? 0) === 1 ? "" : "s"} across the SKYCOT
            marketplace.
          </p>
        </section>

        <div style={{ marginTop: 24 }}>
          <Link href="/skills" className="stub-back">
            ← Back to the marketplace
          </Link>
        </div>
      </div>
    </>
  );
}
