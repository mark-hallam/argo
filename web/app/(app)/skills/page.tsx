"use client";
import "./page.css";

import Link from "next/link";
import { useMemo, useState } from "react";
import { Briefcase, Mail, Search, Sparkles, TrendingUp, Wrench } from "lucide-react";
import type { ReactNode } from "react";
import { useSkills, useSkillsLearnedThisMonth } from "@/lib/hooks/use-skills";
import type { SkillRow } from "@/lib/api/skills";

/**
 * Skills marketplace landing — locked A1 marketplace design language.
 *
 * Brand-loud-marketplace register per the 2026-05-19 design lock memory.
 * Hero search + "try" chips + featured-skills grid. v1 ships the curated
 * "TRY" chip set; cards now read from the live gateway skill registry
 * (`useSkills` → `/skills` envelope) with a seed-fallback when the
 * gateway is empty or unreachable in dev.
 */

const TRY_CHIPS = [
  "approval routing",
  "invoice extraction",
  "candidate screening",
  "deal qualification",
  "support triage",
  "lead enrichment",
];

const ICON_ROTATION: ReactNode[] = [
  <Sparkles size={20} key="sparkles" />,
  <Wrench size={20} key="wrench" />,
  <Briefcase size={20} key="briefcase" />,
  <TrendingUp size={20} key="trending" />,
  <Mail size={20} key="mail" />,
];

/** Title-case a skill name like `approval.route` → "Approval route". */
function prettifySkillName(name: string): string {
  const cleaned = name.replace(/[._-]+/g, " ").trim();
  if (cleaned.length === 0) return name;
  return cleaned.charAt(0).toUpperCase() + cleaned.slice(1);
}

/** Map a skill row to the cohort badge shown on the card. */
function cohortLabel(skill: SkillRow): string {
  const head = skill.name.split(/[.:]/)[0] ?? skill.name;
  return prettifySkillName(head);
}

export default function SkillsPage() {
  const { data, isLoading, isError, error } = useSkills();
  const learnedQuery = useSkillsLearnedThisMonth();
  const skills = useMemo(() => data?.skills ?? [], [data?.skills]);
  const [query, setQuery] = useState("");
  const [learnedOnly, setLearnedOnly] = useState(false);

  const learnedIdSet = useMemo(() => {
    const ids = learnedQuery.data?.learned.skill_ids ?? [];
    return new Set(ids);
  }, [learnedQuery.data?.learned.skill_ids]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    let rows = skills;
    if (learnedOnly) {
      rows = rows.filter((s) => learnedIdSet.has(s.id));
    }
    if (q.length === 0) return rows;
    return rows.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.layer.toLowerCase().includes(q),
    );
  }, [skills, query, learnedOnly, learnedIdSet]);

  return (
    <>
      <div className="skills-stage">
        <div className="skills-eyebrow">
          <span className="skills-eyebrow-dot" />
          MARKETPLACE
        </div>

        <h1 className="skills-hero">
          Every skill your team
          <br />
          will ever <strong>need</strong>.
        </h1>

        <p className="skills-sub">
          Drop in a pre-built skill — approval routing, invoice extraction, candidate screening,
          and dozens more — and your AI teammates start using it the same shift.
        </p>

        <div className="search-row">
          <div className="search-icon">
            <Search size={16} />
          </div>
          <input
            type="text"
            className="search-input"
            placeholder="Search skills, cohorts, or workers…"
            aria-label="Search the skill marketplace"
            data-testid="skills-search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <span className="count-pill" data-testid="skills-count">
            {skills.length} skills · {filtered.length} shown
          </span>
          <span className="search-kbd">⌘K</span>
        </div>

        <div className="try-row">
          <span className="try-label">TRY</span>
          {TRY_CHIPS.map((chip) => (
            <button
              type="button"
              key={chip}
              className="try-chip"
              tabIndex={0}
              onClick={() => setQuery(chip)}
            >
              {chip}
            </button>
          ))}
          <button
            type="button"
            className={`try-chip${learnedOnly ? " is-active" : ""}`}
            tabIndex={0}
            data-testid="skills-filter-learned-this-month"
            aria-pressed={learnedOnly}
            onClick={() => setLearnedOnly((v) => !v)}
          >
            Skills my workers learned this month
          </button>
        </div>

        <div className="section-label">FEATURED SKILLS</div>

        {isLoading && (
          <div className="state-panel" data-testid="skills-loading">
            Loading skills from the marketplace…
          </div>
        )}

        {isError && !isLoading && (
          <div
            className="state-panel state-panel-error"
            data-testid="skills-error"
            role="alert"
          >
            Couldn’t reach the skill registry
            {error instanceof Error ? `: ${error.message}` : "."}
          </div>
        )}

        {!isLoading && !isError && filtered.length === 0 && (
          <div className="state-panel" data-testid="skills-empty">
            No skills match “{query}”. Try a different search or clear the filter.
          </div>
        )}

        {!isLoading && !isError && filtered.length > 0 && (
          <div className="featured-grid">
            {filtered.map((skill, idx) => (
              <Link
                key={skill.id}
                href={`/skills/${encodeURIComponent(skill.name)}`}
                className="skill-card"
                data-testid="skill-card"
              >
                <div className="skill-icon">
                  {ICON_ROTATION[idx % ICON_ROTATION.length]}
                </div>
                <h2 className="skill-title">{prettifySkillName(skill.name)}</h2>
                <p className="skill-blurb">{skill.description}</p>
                <div className="skill-meta">
                  <span className="skill-meta-item skill-cohort">{cohortLabel(skill)}</span>
                  <span className="skill-meta-item">v{skill.version}</span>
                  <span className="skill-meta-item">{skill.layer}</span>
                </div>
                <div className="skill-id">{skill.name}</div>
              </Link>
            ))}
          </div>
        )}
      </div>
    </>
  );
}
