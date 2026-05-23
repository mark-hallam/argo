# web-customers-content-hipaa-soften

**Authored against:** `origin/main` 14f44211d on 2026-05-24.
**Validate before claim:** `bash scripts/check-brief-freshness.sh web-customers-content-hipaa-soften`.

## Why this brief exists

Surfaced as residual over-claim from PR #2125 (`eval-compliance-claims-substrate`, MERGED 2026-05-23). The `/customers` and customer-detail surfaces use HIPAA-shaped language (e.g. "HIPAA compliant", "patient data ready", "PHI-safe") that implies a current state of compliance the substrate does not yet support. The 2026-05-20 wave-3 audit (`docs/skycot/audit/wave-3-2026-05-20/`) flagged similar HIPAA over-claims on /landing and /security; this brief closes the residual on the `/customers` surface that wave-3 missed.

Sibling brief `web-soc2-attestation-strip-soften` (MERGED #2154) softened the SOC 2 attestation language; this is the HIPAA companion.

## Acceptance criteria

1. Grep `web/app/` for HIPAA-implying language on customer-facing surfaces:
   - `web/app/customers/...` (case-study cards, vertical-specific copy)
   - `web/app/for/...` (the per-vertical landings — the healthcare vertical especially)
   - Any sample-data / fixture files that render to the customers grid
2. Replace the louder register with the operator-locked softened form. The pattern from `web-soc2-attestation-strip-soften` is the model: e.g. "HIPAA controls in place — Business Associate Agreement available on request" rather than "HIPAA compliant".
3. Confirm the softened phrasing aligns with `/security` page truth-source (claim-time worker reads `/security` first and reuses verbatim where possible).
4. No new instances of the louder register on any marketing surface.
5. `pnpm typecheck` + `pnpm lint` clean.

## Parallel-surface declarations

- No new code, no new pub surface. Copy change only.
- Mirror of the `web-soc2-attestation-strip-soften` pattern landed 2026-05-24.

## May-touch

- `web/app/customers/page.tsx` (and any sub-pages)
- `web/app/for/<healthcare-vertical>/page.tsx` (if HIPAA copy is present)
- Sample-data fixtures (`web/lib/customers/sample-data.ts` or similar — claim-time worker confirms)
- One snapshot test if the customers grid has Playwright/Vitest snapshots.

## Boundary

- Do NOT touch `/security` page — it already has the truth-source phrasing.
- Do NOT remove HIPAA references entirely — controls ARE in place; the over-claim is the implied current-state compliance.
- Do NOT touch SOC 2 / GDPR / ISO 27001 surfaces — separate briefs.

## Sizing

~10-30 LOC (one copy change across ~3-5 files). Fast-mode candidate.

**🟡 P2 (TRUST OVER-CLAIM)** — operator-locked register from 2026-05-20 wave-3 copy audit + PR #2125 finding (deferred residual).
