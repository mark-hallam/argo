# mission-demo-runbook-trust-language-soften

**Authored against:** `origin/main` 14f44211d on 2026-05-24.
**Validate before claim:** `bash scripts/check-brief-freshness.sh mission-demo-runbook-trust-language-soften`.

## Why this brief exists

Surfaced as residual over-claim from PR #2125 (`eval-compliance-claims-substrate`, MERGED 2026-05-23). The v1 demo recording runbook at `docs/skycot/runbook/v1-demo-recording.md` (and any sibling demo scripts under `docs/skycot/demos/` / `docs/skycot/runbook/`) carries trust-laden language ("certified", "compliant", "attested") that does not match the truth-source on `/security` or the softened marketing register that wave-3 landed.

If the demo is recorded and shipped to a prospect with the louder register, the marketing-vs-product trust gap widens. Sibling marketing surfaces have already been softened (`web-soc2-attestation-strip-soften` MERGED #2154); this brief brings the operator-facing demo runbook into alignment so the recorded narration matches.

## Acceptance criteria

1. Grep `docs/skycot/runbook/` + `docs/skycot/demos/` + `docs/skycot/demo_script.md` for:
   - "certified" / "attested" / "compliant" (when used as state-of-being rather than aspirational)
   - "SOC 2 Type II" (without softening qualifier)
   - "HIPAA compliant" / "PHI-safe"
   - "GDPR Article 30 ROPA" (until the substrate ships, see `mission-article-30-ropa-doc-or-primitive`)
2. Replace with operator-locked softened forms, matching `/security` truth-source and the wave-3 register (e.g. "SOC 2 Type II controls in place — external attestation in progress").
3. Add a 1-line front-matter note to the runbook: `Trust register reviewed: 2026-05-24 (operator-locked-soften per wave-3 audit + PR #2125)` so future edits know the audit happened.
4. Optional: add a CI/pre-commit lint that greps for the louder register in `docs/skycot/runbook/` and fails if it reappears — but only if the worker has confidence in the regex; otherwise defer to a separate brief.

## Parallel-surface declarations

- Docs-only change. No code touched. No new pub surface.
- Mirror of `web-soc2-attestation-strip-soften` (MERGED #2154) pattern applied to the runbook docs.

## May-touch

- `docs/skycot/runbook/v1-demo-recording.md`
- `docs/skycot/demos/*.md` (if any)
- `docs/skycot/demo_script.md`
- Any sibling runbook under `docs/skycot/runbook/` that references trust language

## Boundary

- Do NOT touch the source-of-truth files under `crates/argo-compliance/` — those are technical, not marketing.
- Do NOT remove all compliance references — controls ARE in place; the over-claim is the implied attestation state.
- Do NOT touch `web/` surfaces — separate briefs (`web-soc2-attestation-strip-soften`, `web-customers-content-hipaa-soften`).

## Sizing

~10-30 LOC (one copy change across ~3-5 doc files). Fast-mode candidate.

**🟡 P2 (TRUST OVER-CLAIM)** — operator-locked register from 2026-05-20 wave-3 copy audit + PR #2125 finding (deferred residual). Demo-blocking: if shipping demo MP4 to enterprises this week, brings recorded narration into alignment with /security truth-source.
