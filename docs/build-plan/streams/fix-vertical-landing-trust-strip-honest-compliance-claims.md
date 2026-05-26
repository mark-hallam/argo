# fix-vertical-landing-trust-strip-honest-compliance-claims

**Owner**: claude.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-vertical-landing-trust-strip-honest-compliance-claims`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/whole-site-mvp-claims-coverage.md` F6 [P1]).

🟠 **P1 — `/for/[vertical]` pages render the trust strip `GDPR · HIPAA · AFSL` on every vertical landing page regardless of vertical. HIPAA (US healthcare PHI rule) and AFSL (Australian Financial Services License) are regulated, specific claims. They appear on `/for/sales`, `/for/marketing`, `/for/recruiting`, `/for/ops` where the vertical has NO plausible relationship to PHI or AFSL-regulated activity. Meanwhile `/security` is honest: "SOC 2 attestation in progress", with no HIPAA / AFSL claim. The vertical claims contradict the security page AND are legally adventurous (HIPAA/AFSL are not currently held).**

## Root cause

`web/app/for/_components/VerticalLandingShell.tsx:631`:

```tsx
<TrustStrip items={["GDPR", "HIPAA", "AFSL"]} />
```

Rendered identically on every `/for/<vertical>` page. The same component sources verticals from `web/app/for/_data/verticals.ts` and produces 6 vertical pages: `sales`, `marketing`, `recruiting`, `ops`, `support`, `finance`. None of them differentiate the compliance claim.

Cross-reference: `web/app/security/page.tsx` is contradictorily honest — search returns NO `HIPAA` / `ISO 27001` / `AFSL` claim there. The truth-line is "SOC 2-ready audit trail" + "External SOC 2 Type II attestation is in progress".

Per operator-locked memory `feedback_no_theater_in_demo` + the wider truth-boundary discipline: function-side over-claims must not be made. Compliance claims are FUNCTION-side (legal risk if the company asserts HIPAA without having undergone the audit + signed BAAs).

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/whole-site-mvp-claims-coverage.md` F6 [P1] — Vertical landing pages over-claim compliance: `GDPR · HIPAA · AFSL` on every page

## Fix recipe

1. **Source compliance claims per-vertical from `web/app/for/_data/verticals.ts`**, not hardcoded in the shell. Add a `compliance: string[]` field on each `Vertical` definition.

2. **Per-vertical honest claim set** (suggested defaults; operator may tighten):

   | Vertical | Honest claims |
   |---|---|
   | `sales` | `SOC 2-ready audit trail · GDPR-aligned data export` |
   | `marketing` | `SOC 2-ready audit trail · GDPR-aligned data export` |
   | `recruiting` | `SOC 2-ready audit trail · GDPR-aligned data export · EEOC-aware policy gates` |
   | `support` | `SOC 2-ready audit trail · GDPR-aligned data export` |
   | `ops` | `SOC 2-ready audit trail · GDPR-aligned data export` |
   | `finance` | `SOC 2-ready audit trail · GDPR-aligned data export · SOX-aligned audit retention` |

3. **Drop HIPAA and AFSL** from every vertical until the company actually holds them (Business Associate Agreement signed for HIPAA; AFSL granted from ASIC). Add them BACK when true.

4. **Add a healthcare vertical** later if the operator wants HIPAA messaging — that's the only honest place for it.

5. **Cross-validate `/security` consistency**: any compliance claim on a vertical page must also appear on `/security`. Add a vitest regression that fails if any TrustStrip item isn't present in the `/security` page's claim list.

## Acceptance criteria

- [ ] `grep -n "HIPAA\|AFSL" web/app/for/` returns zero matches.
- [ ] Each `/for/<vertical>` page's TrustStrip is sourced from the matching `Vertical.compliance` field — not hardcoded in the shell.
- [ ] Every TrustStrip item is consistent with what `/security` claims (no vertical claims something /security doesn't back).
- [ ] `pnpm --dir web exec tsc --noEmit` green.
- [ ] `pnpm --dir web exec vitest run app/for` passes; add a regression that asserts no `/for/<vertical>` claims `HIPAA` / `AFSL` / `ISO27001` unless `/security` also lists them.
- [ ] Operator legal-review checkpoint: post-fix copy reviewed for any other surface that may have inherited the over-claim (e.g. `/customers`, `/pricing` enterprise tier).

## May-touch

- `web/app/for/_components/VerticalLandingShell.tsx` (~5 LOC — read from data not hardcoded)
- `web/app/for/_data/verticals.ts` (~60 LOC — add `compliance` field × 6 verticals)
- `web/app/security/page.tsx` (verify-only — no edits unless mismatched)
- New `web/lib/__tests__/compliance-claims-consistency.test.ts` (~50 LOC)
- `web/app/customers/<slug>/page.tsx` files — verify whether any case-study repeats the HIPAA claim (covered by `web-customers-content-hipaa-soften` as well)

## Out of scope

- The "2,847 teams this month" social-proof number on the same shell (operator-flagged as dressing, OK for POC; ws-F17)
- The `/customers` case-study HIPAA-soften (covered by `web-customers-content-hipaa-soften`)
- Real SOC 2 attestation work itself (eng-org program, not a UX brief)
- Adding a healthcare vertical (operator-decisional)

## Effort

S. ~80 LOC + ~50 LOC test. ~2 hours. P1 because (a) legal risk on HIPAA/AFSL over-claims and (b) credibility risk when an investor flips from /for/sales → /security and sees the inconsistency.
