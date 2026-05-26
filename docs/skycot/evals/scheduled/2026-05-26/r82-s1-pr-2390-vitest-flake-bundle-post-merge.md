# R82 · S1 · PR #2390 vitest flake bundle — POST-MERGE anti-theater

**Date**: 2026-05-26 (round 82, scheduled-task hourly fire ~07:10Z / 17:10 AEST)
**Pillar**: Pillar III — observability + repeatability (test-fix PRs are top theater risk)
**Verdict**: 🟢 **PASS · P2** (no severity gap; production component changes are real + assertions are durable scan guards, not single-test rewrites)

## Setup

PR #2390 — `web: fix pre-existing vitest flake bundle` — MERGED 2026-05-26T06:42:09Z (commit `8734686c8`). +90 / −55 across 5 files: SubstrateChip component +2 / −2 + 4 test files. The PR body claims "207 files / 1574 tests passed" after the fix. The anti-theater question this round: **did the production component actually change to match the assertions, or did only the tests get rewritten to match the existing component?**

Commands:

```
cd /tmp/eval-r82-wt
git show 8734686c8 -- web/components/skycot/workflow/SubstrateChip.tsx
rg -n "#71717a|#52525b" web/__tests__/a11y-contrast.test.ts web/components/skycot/workflow/SubstrateChip.tsx
rg -n "ease-in-out|motion-ease" web/__tests__/brand-execution-polish.test.tsx
sed -n '113,130p' web/__tests__/a11y-contrast.test.ts
```

## Observed

Two real production changes in `web/components/skycot/workflow/SubstrateChip.tsx`:

| Line | Before | After | Why |
|---|---|---|---|
| 30 (headless meta) | `dot: "#71717a"` | `dot: "#52525b"` | Slate-500 → slate-600. Improves contrast against `#fafaff` from ~4.4:1 → ~7.0:1 (WCAG AAA). |
| 94 (live-pulse keyframe) | `1.15s ease-in-out infinite` | `1.15s var(--motion-ease) infinite` | Uses the design-token motion easing per `DESIGN_TOKENS.md`. |

The guard that catches `#71717a` regressions lives at `web/__tests__/a11y-contrast.test.ts:113-130` (re-read in full):

```ts
it("no hardcoded #71717a remains in web/ (regression guard — fails AA against #fafaff)", () => {
    const offenders: string[] = [];
    const re = /#71717[aA]\b/;
    for (const file of walk(SCAN_ROOT)) {
      const text = readFileSync(file, "utf8");
      const lines = text.split("\n");
      lines.forEach((line, i) => {
        if (re.test(line)) offenders.push(`${file}:${i + 1} → ${line.trim()}`);
      });
    }
    expect(
      offenders,
      "Hex #71717a (~4.4:1 on #fafaff) fails WCAG AA. Use #52525b (slate-600, 7.0:1) or var(--muted-foreground).",
    ).toEqual([]);
});
```

The analogous `ease-in-out` scan guard lives at `web/__tests__/brand-execution-polish.test.tsx:48` with the message `Use ease-[var(--motion-ease)] per DESIGN_TOKENS.md motion contract`.

The other four test-file updates (`a11y-headings.test.ts`, `approvals/page.test.tsx`, `settings/page.test.tsx`, `workflows/sub-workflow-node.test.tsx`) are legitimate alignments against current production behavior:
- settings page now delegates to `SettingsLayout` with a workspace panel
- sub-workflow picker is wrapped in `QueryClientProvider` (test was missing the provider)
- approvals dashboard correctly renders `approval-rate-empty` when all `SAMPLE_APPROVALS` are pending (the populated-ring path is tracked by sibling brief `fix-approvals-seed-decided-approvals-for-ring`)
- a11y-headings now recognises literal `<h1>` headings from route layouts (test was over-strict before)

## Anti-theater check

The two key guards — `#71717a` ban and `ease-in-out` ban — are **directory-wide regex scans** that walk every file under `web/`. They are NOT single-file `expect(component.dot).toBe("#52525b")` assertions that could be silently rewritten. If anyone re-hardcodes `#71717a` anywhere in `web/`, the test fails with a list of offending file:line pairs. This is a durable property guard, not a self-confirming test.

The component change is also visible in the SAMPLE_TEAM mock dataset — Maya's substrate chip on the live `/api/v1/team` payload renders through this component, so the colour shift will be observable in any future browser screenshot pass.

## Gap / follow-up

None. PR is well-bounded and the test guards are property-style, not snapshot-style. No brief recommended.

The sibling brief `fix-approvals-seed-decided-approvals-for-ring` is acknowledged in the PR body — it carries the legitimate test-data follow-up to exercise the populated-ring path. Already CLAIMED on the ledger (per git log `b45b47cda`); no evaluator action needed.
