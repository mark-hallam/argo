# Approvals page — brand register decision

**Status**: needs operator decision
**Surfaced by**: `web-brand-drift-cleanup` (2026-05-21)
**Origin**: `docs/skycot/audit/brand/2026-05-20-wave-2-brand-consistency.md`,
finding #5 "Register mismatches between lock and implementation"

## The conflict

The 2026-05-20 wave-2 brand-consistency audit flagged a mismatch:

- **Operator memory** (`project_approvals_page_design_locked.md`,
  locked 2026-05-19): "Approvals inbox design locked — E-ops-dashboard
  … **enterprise-restrained register (Inter only)**, no Audiowide on
  this page, gradient sparingly."

- **Audit claim** (wave-2): "implementation uses `<BrandLoudHero>`
  (Audiowide + gradient banner). Either the lock memory is stale or
  the implementation drifted. Worth a 1-line operator confirmation."

## Investigation result (cleanup-brief author)

Inspected `web/app/(app)/page.tsx` end-to-end while authoring this
note. The page does NOT use `<BrandLoudHero>`. The header surface is
the local `PageHeader` component (lines 404–482), which renders:

- Plain `<h1 className="text-xl font-semibold tracking-tight">` with
  no Audiowide.
- shadcn theme tokens throughout (`var(--background)`,
  `var(--foreground)`, `var(--primary)`).
- No `linear-gradient(...)` anywhere in the file.
- No `<link>` to the Audiowide Google Font.

`grep` for `BrandLoudHero` across `web/app/(app)/` returns exactly two
matches: `team/page.tsx` and `cohorts/page.tsx`. Neither is approvals.

Conclusion: the audit's specific claim is **stale or mistaken**. The
approvals page is already in the brand-restrained register that the
operator memory locks. There is no actual register conflict in the
shipped code at `0aaa94b4`.

## What still needs operator confirmation

Even though the implementation is already restrained, two adjacent
questions are open and worth a one-line operator call:

1. **Is the restrained lock still load-bearing?** D3 brand-consistency
   pass (memory `project_brand_consistency_pass.md`, 2026-05-19) is
   noted to SUPERSEDE the "Inter only / no Audiowide" claims in some
   individual design-lock memories. Does that supersedure apply to
   `/approvals`, or is approvals carved out of D3 as the
   enterprise-trust surface that stays restrained on purpose?

2. **Should the audit be corrected?** The wave-2 audit's claim about
   `<BrandLoudHero>` on `/approvals` is incorrect. Leaving it
   uncorrected risks future cleanup briefs trying to "fix" something
   that isn't broken. Suggest a one-line erratum at the top of the
   audit doc once operator confirms #1.

## Recommendation (not a decision)

If the operator confirms the locked register is correct as-is, this
file can be closed as resolved and a one-line erratum appended to
`docs/skycot/audit/brand/2026-05-20-wave-2-brand-consistency.md`.
If the operator wants `/approvals` to adopt the D3 brand-loud recipe
(Audiowide eyebrow + purple→pink banner) for consistency with
`/cohorts` and `/team`, a follow-up brief should target `PageHeader`
in `web/app/(app)/page.tsx` and the `project_approvals_page_design_locked.md`
memory should be updated.

## Out of scope for this brief

The cleanup brief (`web-brand-drift-cleanup`) is restricted to the
two surgical edits on `/settings/members` and `/landing`. It must
NOT touch `web/app/(app)/page.tsx` or any approvals component
rendering. This decision doc is the artifact the brief is responsible
for producing; the actual remediation (if any) belongs to a separate
brief once the operator decides.
