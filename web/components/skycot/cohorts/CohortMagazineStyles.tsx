"use client";

/**
 * Locked D3 magazine-tile styles for /cohorts. Wraps the Audiowide +
 * JetBrains Mono font import (Inter already loads from the root layout)
 * and the editorial tile geometry — lead/stacked/compact tiles, dept
 * tabs, section header. Honours `prefers-reduced-motion`: the lead-tile
 * hover lift and shine animations no-op for users with the OS pref.
 *
 * Why a dedicated styles component rather than a CSS Module: the page
 * is a client component (uses `useState` for dept tabs + the drawer);
 * styled-jsx with `global` co-locates the locked design tokens next to
 * the markup that consumes them and matches the existing brand-loud
 * pattern in `BrandLoudHero` and `CelebrationBanner`.
 */
export function CohortMagazineStyles() {
  return (
    <>
      <style jsx global>{`
        :root {
          --skycot-cohort-purple: #8b5cf6;
          --skycot-cohort-pink: #ec4899;
          --skycot-cohort-coral: #ff6b6b;
          --skycot-cohort-gradient: linear-gradient(
            135deg,
            #8b5cf6,
            #ec4899
          );
          --skycot-cohort-gradient-3: linear-gradient(
            135deg,
            #8b5cf6,
            #ec4899,
            #ff6b6b
          );
          --skycot-cohort-ink: #1e1b4b;
          --skycot-cohort-ink-2: #3f3f46;
          --skycot-cohort-ink-3: #52525b;
          --skycot-cohort-ink-4: #a1a1aa;
          --skycot-cohort-line: #ece9f5;
          --skycot-cohort-line-2: #d1d5db;
          --skycot-cohort-paper-2: #f3f4f6;
          --skycot-cohort-bg: #fafaff;
        }

        /* ── DEPT TABS ─────────────────────────────────────────── */
        .skycot-cohort-tabs {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
          gap: 10px;
          margin-bottom: 28px;
        }
        .skycot-cohort-tab {
          appearance: none;
          padding: 16px 14px;
          border-radius: 14px;
          background: var(--skycot-cohort-bg);
          border: 1px solid var(--skycot-cohort-line);
          cursor: pointer;
          text-align: center;
          transition: transform 0.15s, border-color 0.15s, box-shadow 0.15s,
            background 0.15s;
          color: var(--skycot-cohort-ink);
          font-family: inherit;
        }
        .skycot-cohort-tab:hover {
          transform: translateY(-2px);
          border-color: var(--skycot-cohort-line-2);
        }
        .skycot-cohort-tab.is-on {
          border-color: var(--skycot-cohort-purple);
          box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.08),
            0 8px 20px -6px rgba(139, 92, 246, 0.2);
          background: linear-gradient(
            180deg,
            rgba(139, 92, 246, 0.04),
            rgba(139, 92, 246, 0.02)
          );
        }
        .skycot-cohort-tab-ico {
          display: grid;
          place-items: center;
          width: 36px;
          height: 36px;
          margin: 0 auto 6px;
          border-radius: 10px;
          background: var(--skycot-cohort-paper-2);
          font-size: 18px;
        }
        .skycot-cohort-tab.is-on .skycot-cohort-tab-ico {
          background: var(--skycot-cohort-gradient);
        }
        .skycot-cohort-tab-name {
          display: block;
          font-size: 13px;
          font-weight: 600;
          letter-spacing: -0.005em;
        }
        .skycot-cohort-tab-ct {
          display: block;
          margin-top: 2px;
          font-family: var(--font-mono);
          font-size: 10.5px;
          font-weight: 600;
          color: var(--skycot-cohort-ink-3);
        }
        .skycot-cohort-tab.is-on .skycot-cohort-tab-ct {
          color: var(--skycot-cohort-purple);
        }

        /* ── SECTION HEADER ────────────────────────────────────── */
        .skycot-cohort-mag-section-h {
          display: flex;
          align-items: baseline;
          justify-content: space-between;
          gap: 16px;
          flex-wrap: wrap;
          margin-bottom: 18px;
          padding-bottom: 12px;
          border-bottom: 2px solid var(--skycot-cohort-ink);
        }
        .skycot-cohort-mag-section-h h2 {
          margin: 0;
          font-size: 28px;
          font-weight: 200;
          letter-spacing: -0.025em;
          color: var(--skycot-cohort-ink);
        }
        .skycot-cohort-mag-section-h h2 strong {
          font-weight: 700;
        }
        .skycot-cohort-mag-section-meta {
          font-family: var(--font-mono);
          font-size: 11px;
          color: var(--skycot-cohort-ink-3);
          letter-spacing: 0.14em;
          text-transform: uppercase;
          font-weight: 700;
        }

        /* ── MAGAZINE GRID ─────────────────────────────────────── */
        .skycot-cohort-mag {
          margin-bottom: 32px;
        }
        .skycot-cohort-mag-row1 {
          display: grid;
          grid-template-columns: 2fr 1fr;
          gap: 14px;
          margin-bottom: 14px;
        }
        .skycot-cohort-mag-row1.is-solo {
          grid-template-columns: 1fr;
        }
        .skycot-cohort-mag-rightcol {
          display: flex;
          flex-direction: column;
          gap: 14px;
        }
        .skycot-cohort-mag-row2 {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
          gap: 14px;
        }
        @media (max-width: 720px) {
          .skycot-cohort-mag-row1,
          .skycot-cohort-mag-row1.is-solo {
            grid-template-columns: 1fr;
          }
        }

        /* ── COMMON TILE RESET ─────────────────────────────────── */
        .skycot-cohort-lead-tile,
        .skycot-cohort-stacked-tile,
        .skycot-cohort-compact-tile {
          appearance: none;
          text-align: left;
          font-family: inherit;
          color: var(--skycot-cohort-ink);
          background: var(--skycot-cohort-bg);
          border: 1px solid var(--skycot-cohort-line);
          border-radius: 16px;
          padding: 22px;
          cursor: pointer;
          display: flex;
          flex-direction: column;
          position: relative;
          transition: transform 0.15s, border-color 0.15s, box-shadow 0.15s;
          width: 100%;
        }
        .skycot-cohort-lead-tile:hover,
        .skycot-cohort-stacked-tile:hover,
        .skycot-cohort-compact-tile:hover {
          transform: translateY(-2px);
          border-color: var(--skycot-cohort-line-2);
          box-shadow: 0 8px 20px -6px rgba(0, 0, 0, 0.08);
        }
        .skycot-cohort-lead-tile:focus-visible,
        .skycot-cohort-stacked-tile:focus-visible,
        .skycot-cohort-compact-tile:focus-visible {
          outline: 2px solid var(--skycot-cohort-purple);
          outline-offset: 2px;
        }
        .skycot-cohort-lead-tile[data-selected="true"],
        .skycot-cohort-stacked-tile[data-selected="true"],
        .skycot-cohort-compact-tile[data-selected="true"] {
          border-color: var(--skycot-cohort-purple);
          box-shadow: 0 0 0 2px rgba(139, 92, 246, 0.18);
        }

        /* ── LEAD TILE ─────────────────────────────────────────── */
        .skycot-cohort-lead-tile {
          background: linear-gradient(
            135deg,
            rgba(236, 72, 153, 0.06),
            rgba(255, 107, 107, 0.06)
          );
          border: 1px solid rgba(236, 72, 153, 0.18);
          padding: 32px;
          min-height: 100%;
        }
        .skycot-cohort-lead-badge {
          position: absolute;
          top: 16px;
          right: 16px;
          padding: 4px 10px;
          border-radius: 4px;
          font-family: var(--font-mono);
          font-size: 9.5px;
          font-weight: 700;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          color: white;
          background: var(--skycot-cohort-gradient);
        }
        .skycot-cohort-lead-av {
          width: 80px;
          height: 80px;
          border-radius: 20px;
          margin-bottom: 22px;
          display: grid;
          place-items: center;
          color: white;
          font-family: var(--font-audiowide);
          font-weight: 400;
          font-size: 24px;
          letter-spacing: 0.04em;
          background: var(--skycot-cohort-gradient);
        }
        .skycot-cohort-lead-kicker {
          font-family: var(--font-mono);
          font-size: 10px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          color: var(--skycot-cohort-ink-3);
          margin-bottom: 6px;
        }
        .skycot-cohort-lead-title {
          font-family: var(--font-audiowide);
          font-size: 36px;
          font-weight: 400;
          letter-spacing: -0.005em;
          line-height: 1.05;
          margin: 0 0 14px;
          color: var(--skycot-cohort-ink);
          text-transform: capitalize;
        }
        .skycot-cohort-lead-title strong {
          font-weight: 400;
          background: var(--skycot-cohort-gradient);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .skycot-cohort-lead-desc {
          font-size: 15.5px;
          line-height: 1.55;
          color: var(--skycot-cohort-ink-2);
          max-width: 480px;
          margin: 0 0 24px;
          flex: 1;
        }
        .skycot-cohort-lead-stats {
          display: flex;
          gap: 24px;
          flex-wrap: wrap;
          padding-top: 18px;
          border-top: 1px solid var(--skycot-cohort-line);
          margin-bottom: 14px;
          font-family: var(--font-mono);
          font-size: 12px;
          color: var(--skycot-cohort-ink-3);
        }
        .skycot-cohort-lead-stat strong {
          color: var(--skycot-cohort-ink);
          font-weight: 700;
          font-size: 18px;
        }
        .skycot-cohort-lead-stat.is-grad strong {
          background: var(--skycot-cohort-gradient);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .skycot-cohort-lead-foot {
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .skycot-cohort-lead-meta {
          font-size: 12.5px;
          color: var(--skycot-cohort-ink-3);
          font-family: var(--font-mono);
        }

        /* ── STACKED TILE ─────────────────────────────────────── */
        .skycot-cohort-stacked-tile {
          padding: 20px;
        }
        .skycot-cohort-stacked-av {
          width: 44px;
          height: 44px;
          border-radius: 12px;
          margin-bottom: 12px;
          display: grid;
          place-items: center;
          color: white;
          font-family: var(--font-audiowide);
          font-size: 13px;
          letter-spacing: 0.04em;
          background: linear-gradient(135deg, #ec4899, #ff6b6b);
        }
        .skycot-cohort-stacked-kicker {
          font-family: var(--font-mono);
          font-size: 10px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          color: var(--skycot-cohort-ink-3);
          margin-bottom: 4px;
        }
        .skycot-cohort-stacked-title {
          margin: 0 0 6px;
          font-size: 16px;
          font-weight: 700;
          letter-spacing: -0.015em;
          display: flex;
          gap: 8px;
          align-items: baseline;
          flex-wrap: wrap;
        }
        .skycot-cohort-stacked-band {
          color: var(--skycot-cohort-ink);
        }
        .skycot-cohort-stacked-vertical {
          font-weight: 500;
          color: var(--skycot-cohort-ink-3);
          text-transform: capitalize;
        }
        .skycot-cohort-stacked-desc {
          font-size: 13px;
          color: var(--skycot-cohort-ink-2);
          line-height: 1.5;
          margin: 0 0 14px;
          flex: 1;
        }
        .skycot-cohort-stacked-stats {
          display: flex;
          gap: 12px;
          flex-wrap: wrap;
          padding-top: 10px;
          border-top: 1px solid var(--skycot-cohort-line);
          font-family: var(--font-mono);
          font-size: 11px;
          color: var(--skycot-cohort-ink-3);
        }
        .skycot-cohort-stacked-stats strong {
          color: var(--skycot-cohort-ink);
          font-weight: 700;
          font-size: 12px;
        }

        /* ── COMPACT TILE ─────────────────────────────────────── */
        .skycot-cohort-compact-tile {
          padding: 18px;
        }
        .skycot-cohort-compact-av {
          width: 40px;
          height: 40px;
          border-radius: 10px;
          margin-bottom: 12px;
          display: grid;
          place-items: center;
          color: white;
          font-family: var(--font-audiowide);
          font-size: 12px;
          letter-spacing: 0.04em;
          background: linear-gradient(135deg, #14b8a6, #8b5cf6);
        }
        .skycot-cohort-compact-kicker {
          font-family: var(--font-mono);
          font-size: 9.5px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          color: var(--skycot-cohort-ink-3);
          margin-bottom: 4px;
        }
        .skycot-cohort-compact-title {
          margin: 0 0 6px;
          font-size: 15px;
          font-weight: 700;
          letter-spacing: -0.015em;
          display: flex;
          gap: 6px;
          align-items: baseline;
          flex-wrap: wrap;
        }
        .skycot-cohort-compact-band {
          color: var(--skycot-cohort-ink);
        }
        .skycot-cohort-compact-vertical {
          font-weight: 500;
          color: var(--skycot-cohort-ink-3);
          text-transform: capitalize;
        }
        .skycot-cohort-compact-desc {
          font-size: 12.5px;
          color: var(--skycot-cohort-ink-2);
          line-height: 1.5;
          margin: 0 0 12px;
          flex: 1;
          display: -webkit-box;
          -webkit-line-clamp: 3;
          -webkit-box-orient: vertical;
          overflow: hidden;
        }
        .skycot-cohort-compact-stats {
          display: flex;
          gap: 10px;
          flex-wrap: wrap;
          padding-top: 10px;
          border-top: 1px solid var(--skycot-cohort-line);
          font-family: var(--font-mono);
          font-size: 11px;
          color: var(--skycot-cohort-ink-3);
        }
        .skycot-cohort-compact-stats strong {
          color: var(--skycot-cohort-ink);
          font-weight: 700;
        }

        /* ── LOADING SKELETON ──────────────────────────────────── */
        /*
         * Cold-load placeholders for /cohorts. Geometry mirrors the
         * lead/stacked/compact tiles so the asymmetric layout is
         * reserved before data hydrates — no shift when CohortMagazineGrid
         * replaces the skeleton.
         */
        .skycot-cohort-skel {
          border-radius: 16px;
          border: 1px solid var(--skycot-cohort-line);
          background: var(--skycot-cohort-bg);
          padding: 22px;
          display: flex;
          flex-direction: column;
          gap: 10px;
          position: relative;
          overflow: hidden;
        }
        .skycot-cohort-skel-lead {
          padding: 32px;
          min-height: 360px;
          background: linear-gradient(
            135deg,
            rgba(236, 72, 153, 0.06),
            rgba(255, 107, 107, 0.06)
          );
          border: 1px solid rgba(236, 72, 153, 0.18);
        }
        .skycot-cohort-skel-stacked {
          padding: 20px;
          min-height: 170px;
        }
        .skycot-cohort-skel-compact {
          padding: 18px;
          min-height: 178px;
        }
        .skycot-cohort-skel-av {
          border-radius: 12px;
          background: var(--skycot-cohort-paper-2);
          margin-bottom: 4px;
        }
        .skycot-cohort-skel-av-lead {
          width: 80px;
          height: 80px;
          border-radius: 20px;
          margin-bottom: 14px;
        }
        .skycot-cohort-skel-av-stacked {
          width: 44px;
          height: 44px;
        }
        .skycot-cohort-skel-av-compact {
          width: 40px;
          height: 40px;
          border-radius: 10px;
        }
        .skycot-cohort-skel-line {
          height: 10px;
          border-radius: 6px;
          background: var(--skycot-cohort-paper-2);
        }
        .skycot-cohort-skel-kicker {
          width: 42%;
          height: 8px;
        }
        .skycot-cohort-skel-title-1 {
          width: 72%;
          height: 22px;
          margin-top: 2px;
        }
        .skycot-cohort-skel-title-2 {
          width: 56%;
          height: 16px;
        }
        .skycot-cohort-skel-desc {
          width: 92%;
        }
        .skycot-cohort-skel-desc-2 {
          width: 78%;
        }
        .skycot-cohort-skel-stats {
          margin-top: auto;
          height: 14px;
          width: 62%;
          border-radius: 6px;
          background: var(--skycot-cohort-paper-2);
          border-top: 1px solid var(--skycot-cohort-line);
        }
        .skycot-cohort-skel-sr {
          position: absolute;
          width: 1px;
          height: 1px;
          padding: 0;
          margin: -1px;
          overflow: hidden;
          clip: rect(0, 0, 0, 0);
          white-space: nowrap;
          border: 0;
        }

        /* Shimmer sweep — single moving highlight band. */
        .skycot-cohort-skel-av,
        .skycot-cohort-skel-line,
        .skycot-cohort-skel-stats {
          background-image: linear-gradient(
            90deg,
            var(--skycot-cohort-paper-2) 0%,
            rgba(255, 255, 255, 0.85) 50%,
            var(--skycot-cohort-paper-2) 100%
          );
          background-size: 200% 100%;
          background-repeat: no-repeat;
          background-position: 100% 0;
          animation: skycot-cohort-skel-shimmer 1.4s ease-in-out infinite;
        }
        @keyframes skycot-cohort-skel-shimmer {
          0% {
            background-position: 100% 0;
          }
          100% {
            background-position: -100% 0;
          }
        }

        /* ── A11Y: prefers-reduced-motion ──────────────────────── */
        @media (prefers-reduced-motion: reduce) {
          .skycot-cohort-tab,
          .skycot-cohort-lead-tile,
          .skycot-cohort-stacked-tile,
          .skycot-cohort-compact-tile {
            transition: none !important;
          }
          .skycot-cohort-tab:hover,
          .skycot-cohort-lead-tile:hover,
          .skycot-cohort-stacked-tile:hover,
          .skycot-cohort-compact-tile:hover {
            transform: none !important;
          }
          .skycot-cohort-skel-av,
          .skycot-cohort-skel-line,
          .skycot-cohort-skel-stats {
            animation: none !important;
            background-image: none !important;
            background-color: var(--skycot-cohort-paper-2) !important;
          }
        }
      `}</style>
    </>
  );
}
