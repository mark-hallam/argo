# fix-demo-mode-banner-investor-safe-copy

**Owner**: claude.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-mode-banner-investor-safe-copy`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/scene-1-and-2-claim-1-and-2.md` F8 [P1] + `scene-4-and-5-claim-4-and-5.md` F11 [P1].

🟠 **P1 — `<DemoBanner>` ("Demo mode — showing sample data. Sign in to see your own workspace") fires across every in-app page when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is set. Camera frames the page, banner says "actually it's sample data" — directly undercutting the v1.0 narrative that the demo records REAL data per the NO-THEATER rule.**

## Root cause

`web/components/skycot/DemoBanner.tsx:44-46` renders the literal copy:
> "Demo mode — showing sample data. Sign in to see your own workspace."

The banner is correct for a pre-v1.0 mock-bypass era when the SPA actually rendered fake data. Per the operator-locked memory `feedback_no_theater_in_demo` (2026-05-24), the v1.0 demo MP4 must record REAL backend data — not mocks. With the demo-seed CLI seeding the gateway, the "sample data" copy is now factually wrong.

Two failure modes:
1. **Investor demo (NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 + real seeded data)** — copy lies, undercuts the narrative.
2. **Investor demo (NEXT_PUBLIC_SKYCOT_DEMO_MODE=0 + real seeded data, no banner)** — silent, but loses the "this is a hosted POC, not your prod tenant" disclosure.

Need a third state: "Demo workspace — Skycot's hosted POC. [Talk to sales]" — discloses the POC nature without claiming the data is fake. Discreet pill or banner; close-X stays.

Per the audit: "the camera-visible state is 'I clicked X and Y filtered' without any real cross-tenant learning data underneath" — Scene 4/5 narration competes with the banner.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F8 [P1] — Demo banner reads "showing sample data" on /team
- scene-4-and-5-claim-4-and-5 F11 [P1] — Same banner on /workflows
- whole-site-mvp-claims-coverage F21 [P3] — Banner not fully decided yet (folded — clarify here)

## Fix recipe

1. **Rewrite banner copy** at `web/components/skycot/DemoBanner.tsx:40-50`:
   ```tsx
   <span>
     <strong>Demo workspace</strong> — Skycot's hosted POC. The actions you take here are real but live in a sandboxed tenant.
     <Link href="/contact">Talk to sales</Link> to deploy your own.
   </span>
   ```
   Honest, investor-safe, doesn't claim "sample data".

2. **Add a discreet variant** as fallback: a tiny pill in the page corner (`<DemoPill>`) instead of the full-width banner for pages where the camera will pan (Scene 6's audit drill-in, the /approvals dashboard). Gated by `NEXT_PUBLIC_SKYCOT_DEMO_BANNER_STYLE = "pill" | "banner"`.

3. **Operator-set defaults** for the v1.0 MP4:
   - `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` (banner gating var, still respected)
   - `NEXT_PUBLIC_SKYCOT_DEMO_BANNER_STYLE=pill` (small pill, not full banner)
   - Document in `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-record checklist.

4. **Auto-dismiss after 8s** for the banner variant — gives camera a beat to register the disclosure, then fades. Pill variant stays.

5. **Tests** `web/components/skycot/DemoBanner.test.tsx`:
   - Banner copy does NOT contain "sample data" or "Sign in".
   - Pill variant renders when `BANNER_STYLE=pill`.
   - Auto-dismiss after 8s for banner variant (jest fake timers).

## Acceptance criteria

- [ ] Banner copy no longer contains "sample data" or "Sign in to see your own workspace".
- [ ] New copy reads as a POC disclosure ("hosted POC" / "sandboxed tenant"), with "Talk to sales" CTA.
- [ ] `BANNER_STYLE=pill` env produces a discreet corner pill instead of full banner.
- [ ] Auto-dismiss after 8s on the full banner variant; pill stays visible.
- [ ] Pre-record checklist in v1-mission-mvp-runbook.md updated with the env config.
- [ ] Unit tests cover both variants + dismiss behavior.

## May-touch

- `web/components/skycot/DemoBanner.tsx` (copy + variants)
- `web/components/skycot/DemoPill.tsx` (NEW — small variant)
- `web/lib/skycot/demo-mode.ts` (BANNER_STYLE env read)
- `web/components/skycot/DemoBanner.test.tsx` (NEW or extend)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (pre-record checklist row)

## Out of scope

- `NEXT_PUBLIC_SKYCOT_DEMO_MODE` env wiring itself (covered by `fix-signup-demo-mode-env-flag-and-runbook-prerequisites`)
- Demo-seed CLI population (covered by `fix-demo-seed-cli-for-real-roster`)
- /landing page copy

## Effort

~80-120 LOC + 1 small new component + tests + runbook edit. ~2-3 hours.
