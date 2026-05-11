# Stream E4-i — KpiSparkline + supporting data-viz primitives (extract + extend)

**Owner**: claude.
**Depends on**: E4-b MERGED (initial inline KpiSparkline lives there).
**Authored against**: origin/main `a1ff60bd` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E4-i-kpi-sparkline-primitives`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E4 brief 9 of 9 (LAST in E4).

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Chart library | Hand-rolled SVG primitives — no external charting lib | Same; v1 ships these minimal primitives, v2+ may grow into a curated chart library for richer dashboards | n/a |
| Theming | Uses CSS variables exclusively (var(--primary), var(--muted), tone colors) — auto-themes per tenant | Same | n/a |
| Accessibility | SVG with `<title>` + `<desc>` for screen readers; numeric value rendered alongside the chart | Same | n/a |
| Data refresh | Pure-prop render; parent controls refresh | Same; v1.5 may add a streaming-data variant for live charts | `mission-enterprise-charts-streaming` |
| Visual range | Hardcoded color thresholds (success / warning / muted) | Per-tenant chart-color customization tied to white-label theming | already enforced — vars already swap |

v1 is acceptable: these are visual primitives, not
business logic. No security surface area.

## Why

E4-b inlined the `KpiSparkline` component inside the
Overview tab as a 150-LOC SVG primitive. As Track-E
progresses, additional data-viz primitives are needed:
- The audit timeline (E5) shows event-rate sparklines.
- The operator console (deferred) needs progress bars
  + delta chips.
- The worker profile's KPI block could use mini-bar
  charts for week-over-week comparisons.

This brief **extracts** the inline KpiSparkline and
**extends** it with 3 sibling primitives: ProgressBar,
DeltaChip, MiniBar. All 4 land as a single reusable
module with Storybook stories. Bundle size budget keeps
them ≤ 8KB combined.

## Scope

- Extract `KpiSparkline` from E4-b's inline location
  to `web/components/ui/charts/KpiSparkline.tsx` —
  the canonical home.
- Add 3 sibling chart primitives:
  - **ProgressBar** — thin (~6px tall) progress bar
    with color-by-threshold tone. Used by E4-f's
    Budget section.
  - **DeltaChip** — small chip showing a value
    delta with up/down/flat arrow + semantic color.
    Used by E4-b KPI rows, audit cost summaries.
  - **MiniBar** — vertical bar chart, 5-7 bars
    horizontal layout, ~32×24px. For week-over-
    week / daily-rate visualization.
- Storybook stories for each (5+ states per primitive).
- Update E4-b's inline KpiSparkline to import from
  the new canonical location.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/components/ui/charts/**`
  (NEW directory).

## Files you may touch

- `web/components/ui/charts/KpiSparkline.tsx` — NEW
  (canonical home). ≤ 150 LOC.
- `web/components/ui/charts/ProgressBar.tsx` — NEW.
  ≤ 100 LOC.
- `web/components/ui/charts/DeltaChip.tsx` — NEW.
  ≤ 100 LOC.
- `web/components/ui/charts/MiniBar.tsx` — NEW.
  ≤ 130 LOC.
- `web/components/ui/charts/index.ts` — NEW. Barrel
  export of all 4. ≤ 20 LOC.
- `web/components/ui/charts/utils.ts` — NEW. Shared
  helpers: data normalization, color-by-threshold.
  ≤ 100 LOC.
- `web/stories/charts/KpiSparkline.stories.tsx` —
  NEW. ≤ 150 LOC.
- `web/stories/charts/ProgressBar.stories.tsx` —
  NEW. ≤ 100 LOC.
- `web/stories/charts/DeltaChip.stories.tsx` — NEW.
  ≤ 100 LOC.
- `web/stories/charts/MiniBar.stories.tsx` — NEW.
  ≤ 100 LOC.
- `web/components/skycot/worker/KpiSparkline.tsx` —
  REMOVE (extraction). Import from
  `@/components/ui/charts` instead.
- `web/components/skycot/worker/KpiCard.tsx` —
  narrowly scoped: update import to the new path.
  No other changes.
- `web/components/ui/charts/charts.test.tsx` — NEW.
  ≥ 8 tests covering all 4 primitives. ≤ 300 LOC.

## Files you must NOT touch

- All E1/E2/E3 / earlier-E4 files except the narrow
  KpiCard import path update.
- `web/components/ui/Button.tsx`, `Card.tsx`,
  `Avatar.tsx`, `StatusPill.tsx`, etc. — locked.
- `crates/**` — no backend changes.

## Acceptance criteria

- [ ] `KpiSparkline` lives at
      `web/components/ui/charts/KpiSparkline.tsx`.
- [ ] E4-b's `KpiCard` imports from the new path
      (no behavior change).
- [ ] `KpiSparkline` props:
      ```ts
      interface KpiSparklineProps {
        data: number[];        // 5-30 data points
        tone?: "success" | "warning" | "muted" | "destructive" | "info";
        height?: number;       // default 24
        width?: number;        // default 80
        ariaLabel?: string;
      }
      ```
      - Renders an SVG path between min + max
        normalized to the chart area.
      - Last data point gets a small filled circle.
      - Stroke color uses `var(--${tone})` via the
        theme.
- [ ] `ProgressBar` props:
      ```ts
      interface ProgressBarProps {
        value: number;         // 0-100
        thresholds?: { warning?: number; destructive?: number };
        height?: number;       // default 6
        ariaLabel?: string;
      }
      ```
      - Filled portion uses tone color by threshold
        (default muted < warning < destructive).
- [ ] `DeltaChip` props:
      ```ts
      interface DeltaChipProps {
        value: number;         // numeric delta
        unit?: string;         // "%", "$", etc.
        direction: "up" | "down" | "flat";
        directionMeansGood?: boolean;
        // (true: up = success; false: up = destructive)
      }
      ```
      - Renders arrow + value chip.
      - Color from semantic tone: good = success;
        bad = destructive; flat = muted.
- [ ] `MiniBar` props:
      ```ts
      interface MiniBarProps {
        data: number[];        // 5-7 bars typical
        tone?: "success" | "warning" | "muted" | "destructive" | "info";
        height?: number;       // default 24
        width?: number;        // default 32
        ariaLabel?: string;
      }
      ```
      - Vertical bars normalized to max value.
- [ ] All 4 primitives respect tenant theme (Acme
      orange swap works).
- [ ] All 4 primitives have `<title>` + numeric
      value for screen reader.
- [ ] Storybook stories cover ≥ 5 states each:
      default, tone variants (success/warning/etc.),
      edge cases (empty data, single point, all
      zeros, all max, NaN handling).
- [ ] 8 unit tests pass.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta: net 0
      or negative (extraction → no new code
      shipped; just moved).
- [ ] `pnpm storybook:build` succeeds with all
      stories present.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E4-i-kpi-sparkline-primitives`
- PR title: `web(charts): extract KpiSparkline + add ProgressBar/DeltaChip/MiniBar primitives (E4-i)`

## Notes for the implementer

- **No external chart libraries.** Bundle size
  matters; pure SVG is ~50 lines per primitive and
  good enough for these use cases.

- **Data normalization helper.** In `utils.ts`:
  ```ts
  export function normalize(data: number[], min?: number, max?: number) {
    const lo = min ?? Math.min(...data);
    const hi = max ?? Math.max(...data);
    const range = hi - lo || 1;
    return data.map((v) => (v - lo) / range);
  }
  ```

- **Color-by-threshold helper:**
  ```ts
  export function thresholdTone(
    value: number,
    thresholds: { warning?: number; destructive?: number },
  ): "muted" | "warning" | "destructive" {
    if (thresholds.destructive != null && value >= thresholds.destructive) return "destructive";
    if (thresholds.warning != null && value >= thresholds.warning) return "warning";
    return "muted";
  }
  ```

- **DeltaChip example renderings:**
  ```
  ↗ +12%   (success tone — up arrow, up means good)
  ↘ -3.4   (destructive tone — down arrow, down means good is false)
  → 0       (muted — flat)
  ```

- **MiniBar use case.** Worker profile KPI section
  may render `interviews booked: 12 [ ▁▃█▅▆▂▄ ]`
  showing week-over-week. Bars share color tone via
  the `tone` prop.

- **Accessibility.** Each SVG includes:
  ```xml
  <svg role="img" aria-label={ariaLabel}>
    <title>{ariaLabel}</title>
    ...
  </svg>
  ```
  Plus the numeric value rendered as text alongside
  the chart (visible) — screen readers get both.

- **Storybook addon-themes** is already configured
  via Track C; verify stories render correctly in
  both light + dark + Acme themes.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-charts-streaming` — streaming-
  data variants for live dashboards.

(No sibling backend brief — pure frontend
primitives.)

## E4 slice COMPLETE after this commits

After E4-i merges:
- E4-a (shell)
- E4-b (Overview tab)
- E4-c (Activity tab)
- E4-d (Memory tab)
- E4-e (Skills tab)
- E4-f (Settings tab)
- E4-g (RetrainDialog)
- E4-h (AddResponsibilityDialog)
- E4-i (chart primitives)

= 9 briefs, E4 slice fully authored.

Next: E5 (audit timeline, 8 briefs).
