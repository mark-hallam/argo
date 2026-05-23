# web-soc2-attestation-strip-soften

**Authored against:** `origin/main` e92343c85 on 2026-05-23.
**Validate before claim:** `bash scripts/check-brief-freshness.sh web-soc2-attestation-strip-soften`.

## Why this brief exists

Surfaced as over-claim finding #3 / #4 from PR #2125 (`eval-compliance-claims-substrate`, MERGED 2026-05-23). The marketing landing strip and per-vertical landing pills render "SOC 2 Type II — Audit on request" which implies an external attestation that does not exist. The internal `web/app/security/page.tsx` already states "External SOC 2 Type II attestation is in progress" — so the public-facing badge is louder than the truth-source.

Trust over-claims are operator-locked-soften per the 2026-05-20 wave-3 copy audit; this completes the residual landings flagged in audit finding #15/#16/#18.

## Acceptance criteria

1. Find the SOC 2 attestation badge / chip / strip on the marketing surface:
   - `web/app/landing/...` (one badge string in the trust strip)
   - `web/app/(marketing)/...` per-vertical landings (one pill per page if present)
2. Replace "SOC 2 Type II — Audit on request" register with the operator-locked softened form: "SOC 2 Type II controls in place — external attestation in progress." (Or operator-confirmed equivalent — claim-time worker should grep the security page for the canonical phrasing and reuse verbatim.)
3. Audit that the same phrasing now appears consistently across landing strip + per-vertical pills + `/security` page. No new instances of the louder register on any marketing surface.
4. Web typecheck + lint clean.

## Parallel-surface declarations

- No new code, no new pub surface. Copy change only.
- The `/security` page already carries the truth-source line; this brief brings the louder marketing surfaces into alignment, not the inverse.

## May-touch

- `web/app/landing/page.tsx` (or wherever the trust strip lives — claim-time worker confirms)
- `web/app/(marketing)/<vertical>/page.tsx` (one pill per vertical, ~5 files)
- One snapshot test if the marketing pages have Playwright/Vitest snapshots — update the snapshot.

## Boundary

- Do NOT touch `/security` page — it already has the truth-source phrasing.
- Do NOT remove SOC 2 references entirely — controls ARE in place; the over-claim is the implied external attestation.
- Do NOT touch HIPAA / GDPR / ISO 27001 surfaces — those have their own briefs (`web-customers-content-hipaa-soften`, `mission-article-30-ropa-doc-or-primitive`, `mission-demo-runbook-trust-language-soften`).

## Sizing

~10-20 LOC (one copy change across 5-7 files). Fast-mode candidate.

**🟡 P2 (TRUST OVER-CLAIM)** — operator-locked register from 2026-05-20 wave-3 copy audit + PR #2125 finding #3/#4/#15/#16/#18.
