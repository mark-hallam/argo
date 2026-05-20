"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import { Briefcase, Mail, Search, Sparkles, TrendingUp, Wrench } from "lucide-react";
import type { ReactNode } from "react";
import { useSkills } from "@/lib/hooks/use-skills";
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
  const skills = useMemo(() => data?.skills ?? [], [data?.skills]);
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (q.length === 0) return skills;
    return skills.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.layer.toLowerCase().includes(q),
    );
  }, [skills, query]);

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600&display=swap"
        rel="stylesheet"
      />
      <style jsx>{`
        .skills-stage {
          padding: 48px 32px 96px;
          max-width: 1100px;
          margin: 0 auto;
        }
        .skills-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 18px;
          padding: 5px 14px;
          border-radius: 999px;
          border: 1px solid var(--border, #ece9f5);
          background: var(--background, #fafaff);
        }
        .skills-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--brand-teal, #14b8a6);
          box-shadow: 0 0 6px var(--brand-teal, #14b8a6);
        }
        .skills-hero {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 48px;
          line-height: 1.05;
          font-weight: 400;
          letter-spacing: 0;
          margin: 0 0 14px;
          color: var(--foreground, #1e1b4b);
        }
        .skills-hero strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .skills-sub {
          font-size: 16px;
          font-weight: 300;
          color: var(--muted-foreground, #3f3f46);
          margin: 0 0 28px;
          max-width: 620px;
        }
        .search-row {
          display: flex;
          align-items: center;
          gap: 14px;
          padding: 16px 20px;
          border-radius: 14px;
          border: 1px solid var(--border, #ece9f5);
          background: white;
          box-shadow: 0 1px 0 rgba(0, 0, 0, 0.02);
          margin-bottom: 14px;
        }
        .search-icon {
          width: 28px;
          height: 28px;
          border-radius: 8px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          flex-shrink: 0;
        }
        .search-input {
          flex: 1;
          border: none;
          outline: none;
          font-family: "Inter", sans-serif;
          font-size: 18px;
          color: var(--foreground, #1e1b4b);
          background: transparent;
        }
        .search-input::placeholder {
          color: var(--muted-foreground, #71717a);
        }
        .search-kbd {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          padding: 3px 8px;
          border-radius: 6px;
          background: var(--background, #fafaff);
          border: 1px solid var(--border, #ece9f5);
          color: var(--muted-foreground, #71717a);
        }
        .count-pill {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.08em;
        }
        .try-row {
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
          margin-bottom: 48px;
          font-size: 13px;
        }
        .try-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 6px 0;
          margin-right: 4px;
        }
        .try-chip {
          padding: 6px 12px;
          border-radius: 999px;
          background: white;
          border: 1px solid var(--border, #ece9f5);
          color: var(--foreground, #1e1b4b);
          font-size: 13px;
          cursor: pointer;
          transition: all 0.15s;
        }
        .try-chip:hover {
          border-color: #8b5cf6;
          color: #8b5cf6;
        }
        .featured-grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 16px;
        }
        @media (max-width: 900px) {
          .featured-grid {
            grid-template-columns: repeat(2, 1fr);
          }
        }
        @media (max-width: 600px) {
          .featured-grid {
            grid-template-columns: 1fr;
          }
        }
        .skill-card {
          background: white;
          border: 1px solid var(--border, #ece9f5);
          border-radius: 14px;
          padding: 20px;
          cursor: pointer;
          transition: all 0.18s;
          position: relative;
          overflow: hidden;
          display: block;
          color: inherit;
          text-decoration: none;
        }
        .skill-card:hover {
          border-color: rgba(139, 92, 246, 0.4);
          transform: translateY(-2px);
          box-shadow: 0 12px 24px -10px rgba(139, 92, 246, 0.18);
        }
        .skill-icon {
          width: 36px;
          height: 36px;
          border-radius: 9px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          margin-bottom: 14px;
        }
        .skill-title {
          font-family: "Inter", sans-serif;
          font-weight: 600;
          font-size: 17px;
          color: var(--foreground, #1e1b4b);
          margin: 0 0 6px;
        }
        .skill-blurb {
          font-size: 13.5px;
          color: var(--muted-foreground, #3f3f46);
          line-height: 1.45;
          margin: 0 0 16px;
        }
        .skill-meta {
          display: flex;
          gap: 14px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.06em;
          text-transform: uppercase;
          font-weight: 600;
        }
        .skill-meta-item {
          display: inline-flex;
          align-items: center;
          gap: 4px;
        }
        .skill-cohort {
          color: #8b5cf6;
        }
        .skill-id {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          color: var(--muted-foreground, #a1a1aa);
          margin-top: 14px;
          letter-spacing: 0.04em;
        }
        .section-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--muted-foreground, #71717a);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 14px;
        }
        .state-panel {
          border: 1px dashed var(--border, #ece9f5);
          border-radius: 14px;
          padding: 32px;
          text-align: center;
          color: var(--muted-foreground, #71717a);
          background: var(--background, #fafaff);
        }
        .state-panel-error {
          border-color: rgba(220, 38, 38, 0.35);
          color: #991b1b;
          background: rgba(254, 226, 226, 0.45);
        }
      `}</style>
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
                <h3 className="skill-title">{prettifySkillName(skill.name)}</h3>
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
