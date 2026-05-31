"use client";
import "./page.css";

import Link from "next/link";
import { useMemo, useState } from "react";
import { Search } from "lucide-react";
import { EmptyState } from "@/components/ui/EmptyState";
import { SkillsIllustration } from "@/components/empty-state/illustrations/SkillsIllustration";
import {
  useSkills,
  useSkillSearch,
  useSkillsLearnedThisMonth,
} from "@/lib/hooks/use-skills";
import type { SkillRow, SkillSearchResult } from "@/lib/api/skills";
import { iconForSkill } from "@/lib/api/skill-icons";
import { InstallSkillButton } from "@/components/skycot/skills/InstallSkillButton";

/**
 * Skills marketplace landing — locked A1 marketplace design language.
 *
 * Brand-loud-marketplace register per the 2026-05-19 design lock memory.
 * Hero search + "try" chips + featured-skills grid.
 *
 * Search is now SERVER-SIDE: the hero query + the facet control feed
 * `useSkillSearch` → `GET /api/v1/skills/search`, which runs the SAME
 * ranking engine the marketplace uses (`argo_skill::rank_skills`) over
 * the tenant's REAL skill rows. The facet values (category / trust) are
 * derived by the server from real skill columns — never a hand-authored
 * taxonomy. When no query/facet is active the grid shows the full
 * installed catalog from `useSkills` (so the empty / error / loading
 * states + the learned-this-month chip keep working).
 */

export const TRY_CHIPS = [
  "approval",
  "invoice",
  "candidate",
  "deal",
  "support",
  "lead",
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

/**
 * Map a `SkillRow.layer` value to the decorative chip text on the card.
 *
 * Decorative — not a filter token. Operators reading "LIBRARY" next to
 * the interactive TRY pills above the grid (F07 / ux-audit-2026-05-27-2351)
 * expected it to filter the catalog. It doesn't. Library is the default
 * scope; the real scope filter is the facet control above the grid.
 */
export function layerChipLabel(layer: string): string {
  if (layer === "library") return "Verified";
  return layer;
}

/** Humanize a trust-grade discriminant for the facet pill. */
function trustLabel(grade: string): string {
  return grade
    .split("_")
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join(" ");
}

export default function SkillsPage() {
  // Full installed catalog — drives the total count + empty/error states
  // and the unfiltered grid.
  const catalogQuery = useSkills();
  const learnedQuery = useSkillsLearnedThisMonth();
  const catalog = useMemo(
    () => catalogQuery.data?.skills ?? [],
    [catalogQuery.data?.skills],
  );

  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<string | null>(null);
  const [trustMin, setTrustMin] = useState<string | null>(null);
  const [learnedOnly, setLearnedOnly] = useState(false);

  const searchActive =
    query.trim().length > 0 || category !== null || trustMin !== null;

  // Server-side ranked search. The query is always issued (so the facet
  // values stay populated even on an empty query); the grid only swaps
  // to its results when a query/facet is actually active.
  const searchQuery = useSkillSearch({
    q: query.trim(),
    category: category ?? undefined,
    trustMin: trustMin ?? undefined,
  });

  const facets = searchQuery.data?.response.facets;

  const learnedIdSet = useMemo(() => {
    const ids = learnedQuery.data?.learned.skill_ids ?? [];
    return new Set(ids);
  }, [learnedQuery.data?.learned.skill_ids]);

  // Rows to render: the server-ranked results when a search/facet is
  // active, else the full catalog. The learned-this-month chip then
  // narrows whichever set is showing (client-side intersection).
  const rows: Array<SkillRow | SkillSearchResult> = useMemo(() => {
    const base = searchActive
      ? (searchQuery.data?.response.results ?? [])
      : catalog;
    if (!learnedOnly) return base;
    return base.filter(
      (s) => learnedIdSet.has(s.id) || learnedIdSet.has(s.name),
    );
  }, [
    searchActive,
    searchQuery.data?.response.results,
    catalog,
    learnedOnly,
    learnedIdSet,
  ]);

  const isLoading = catalogQuery.isLoading;
  const isError = catalogQuery.isError;
  const error = catalogQuery.error;

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
            onKeyDown={(e) => {
              // Defend against global ⌘K listener seeing our keystrokes if a
              // mid-render focus blip flips event ordering. The chord itself
              // (Cmd/Ctrl+K) still bubbles so the palette can open from here.
              if (!(e.metaKey || e.ctrlKey)) e.stopPropagation();
            }}
            onKeyUp={(e) => {
              if (!(e.metaKey || e.ctrlKey)) e.stopPropagation();
            }}
          />
          <div className="search-meta">
            <span className="count-pill" data-testid="skills-count">
              {searchActive || learnedOnly
                ? `${rows.length} of ${catalog.length} skills`
                : `${catalog.length} skills`}
            </span>
            <span className="search-kbd">⌘K</span>
          </div>
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

        {/* Facet control — values come from the server's real-column
            facets (category cohort / trust grade). Honest: a facet group
            only renders when the server reported values for it. */}
        {facets && (facets.category.length > 0 || facets.trust.length > 0) && (
          <div className="facet-row" data-testid="skills-facets">
            {facets.category.length > 0 && (
              <div className="facet-group">
                <span className="try-label">CATEGORY</span>
                <button
                  type="button"
                  className={`try-chip${category === null ? " is-active" : ""}`}
                  aria-pressed={category === null}
                  onClick={() => setCategory(null)}
                >
                  All
                </button>
                {facets.category.map((f) => (
                  <button
                    type="button"
                    key={`cat-${f.value}`}
                    className={`try-chip${category === f.value ? " is-active" : ""}`}
                    aria-pressed={category === f.value}
                    data-testid={`skills-facet-category-${f.value}`}
                    onClick={() =>
                      setCategory((c) => (c === f.value ? null : f.value))
                    }
                  >
                    {prettifySkillName(f.value)} ({f.count})
                  </button>
                ))}
              </div>
            )}
            {facets.trust.length > 0 && (
              <div className="facet-group">
                <span className="try-label">TRUST</span>
                <button
                  type="button"
                  className={`try-chip${trustMin === null ? " is-active" : ""}`}
                  aria-pressed={trustMin === null}
                  onClick={() => setTrustMin(null)}
                >
                  Any
                </button>
                {facets.trust.map((f) => (
                  <button
                    type="button"
                    key={`trust-${f.value}`}
                    className={`try-chip${trustMin === f.value ? " is-active" : ""}`}
                    aria-pressed={trustMin === f.value}
                    data-testid={`skills-facet-trust-${f.value}`}
                    onClick={() =>
                      setTrustMin((t) => (t === f.value ? null : f.value))
                    }
                  >
                    {trustLabel(f.value)} ({f.count})
                  </button>
                ))}
              </div>
            )}
          </div>
        )}

        <div className="section-label">FEATURED SKILLS</div>

        {isLoading && (
          <div className="state-panel" data-testid="skills-loading">
            Loading skills from the marketplace…
          </div>
        )}

        {isError && !isLoading && (
          <EmptyState
            data-testid="skills-error"
            illustration={SkillsIllustration}
            headline="The skill registry didn’t answer"
            body={
              error instanceof Error
                ? `We couldn’t reach the gateway: ${error.message}. Your saved skills are safe — try again in a moment.`
                : "We couldn’t reach the gateway. Your saved skills are safe — try again in a moment."
            }
            cta={{ label: "Browse the catalog at home", href: "/" }}
          />
        )}

        {!isLoading && !isError && catalog.length === 0 && (
          <EmptyState
            data-testid="skills-workspace-empty"
            illustration={SkillsIllustration}
            headline="No skills installed yet"
            body="Pick from the curated catalog — approval routing, invoice extraction, candidate screening, and more — and your AI teammates start using it the same shift."
            cta={{ label: "Talk to the team", href: "/team" }}
          />
        )}

        {!isLoading && !isError && catalog.length > 0 && rows.length === 0 && (
          <div className="state-panel" data-testid="skills-empty">
            No skills match your search. Try a different term or clear the filters.
          </div>
        )}

        {!isLoading && !isError && catalog.length > 0 && rows.length > 0 && (
          <div className="featured-grid">
            {rows.map((skill) => {
              const Icon = iconForSkill(skill.name);
              return (
              <Link
                key={skill.id}
                href={`/skills/${encodeURIComponent(skill.name)}`}
                className="skill-card"
                data-testid="skill-card"
              >
                <div className="skill-icon">
                  <Icon size={20} />
                </div>
                <h2 className="skill-title">{prettifySkillName(skill.name)}</h2>
                <p className="skill-blurb">{skill.description}</p>
                <div className="skill-meta">
                  <span className="skill-meta-item skill-cohort">{cohortLabel(skill)}</span>
                  <span className="skill-meta-item">v{skill.version}</span>
                  <span
                    className="skill-meta-item skill-meta-item--decorative"
                    data-testid="skill-layer-chip"
                  >
                    {layerChipLabel(skill.layer)}
                  </span>
                </div>
                <div className="skill-id">{skill.name}</div>
                <InstallSkillButton skillId={skill.id} stopPropagation />
              </Link>
              );
            })}
          </div>
        )}
      </div>
    </>
  );
}
