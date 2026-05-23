# mission-pricing-tier-doc-author

**Owner**: claude or codex.
**Depends on**: nothing.
**Authored against**: origin/main `ca02058c9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pricing-tier-doc-author`.

🟢 **P3 — `/pricing` claims need a written source-of-truth doc.** Source: `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` §76 ("cell (e) of the real-vs-stub matrix is unanswerable without it") + multiple gap-manifest rows that cross-reference `/pricing` claims with no doc to compare against.

`web/app/pricing/page.tsx` advertises specific capabilities per tier — "Multi-channel (Slack / iMessage / Email)" (Pro), "HIPAA workflows" (Enterprise), "SOC 2 Type II audit trail" (recently softened to "SOC 2-READY AUDIT TRAIL" via #2154 — see PR body for the change-of-claim discussion). Multiple eval reports (`eval-channels-v1-config-vs-code-inventory`, `eval-compliance-claims-substrate`) reference the pricing page as the substrate-bearing surface, but `docs/skycot/marketing/PRICING.md` does not exist.

The doc needs to be the operator-facing record of:
1. **What each tier promises** (exact wording from `pricing/page.tsx`, copied verbatim).
2. **Which substrate backs each promise** (which crate / handler / route / test pins the claim).
3. **Verdict per claim** — 🟢 substrate exists + test pins / 🟡 substrate exists + no pinning test / 🔴 promise without substrate.
4. **History of promise changes** — when a tier promise is softened or strengthened (e.g. #2154 SOC 2 softening), the diff is captured here with PR back-cite.

This doc is the missing audit-trail surface for sales-promise honesty. It is what an auditor reads to confirm we don't promise things the code doesn't deliver.

## Scope

1. Read `web/app/pricing/page.tsx` end-to-end. Extract every claim verbatim per tier (Free / Pro / Enterprise / Add-ons).
2. For each claim, identify the substrate by walking the codebase. For channels: `crates/argo-channels/src/`. For compliance: `crates/argo-compliance/` (or absence thereof). For audit-trail: `crates/argo-audit/`. For HIPAA: see `eval-compliance-claims-substrate.md` (MERGED).
3. Render the doc as a single markdown file at `docs/skycot/marketing/PRICING.md` with the structure:
   - **Title + last-updated date + git SHA**.
   - **Per-tier section** — table of claims, substrate citation (file:line), verdict.
   - **Cross-tier rollup** — count of 🟢/🟡/🔴 per tier.
   - **Promise-change history** — recent softenings/strengthenings with PR back-cite (start with #2154 SOC 2 softening + #2161 iMessage dispatcher wire).
4. Update `web/app/pricing/page.tsx` header comment (top-of-file) to point at the new doc as the audit-trail source.
5. Update `eval-channels-v1-config-vs-code-inventory.md` + `eval-compliance-claims-substrate.md` to cite the new doc rather than the page directly.

## Files you may touch

- `docs/skycot/marketing/PRICING.md` — NEW.
- `web/app/pricing/page.tsx` — add a top-of-file `// See docs/skycot/marketing/PRICING.md` comment only; no code changes.
- `docs/skycot/evals/scheduled/2026-05-23/eval-channels-v1-config-vs-code-inventory.md` — update cite (find/replace).
- `docs/skycot/evals/scheduled/2026-05-23/eval-compliance-claims-substrate.md` — update cite.
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — flip §76 cell (e) verdict to 🟢 with new-doc back-cite.

## Files you must NOT touch

- Any `crates/` Rust file. This is a docs-only brief.
- The pricing-page UI itself (text, layout, copy, tiers, prices) — sales decisions are operator-locked.

## Acceptance criteria

- [ ] `docs/skycot/marketing/PRICING.md` exists and lists every claim from `web/app/pricing/page.tsx` verbatim.
- [ ] Every claim has a substrate citation (file:line) or is flagged 🔴.
- [ ] Per-tier 🟢/🟡/🔴 tally rendered as a table.
- [ ] Promise-change history section includes at least 2 entries (SOC 2 softening + iMessage wire).
- [ ] `web/app/pricing/page.tsx` carries a one-line header pointer to the new doc.
- [ ] At least 2 eval reports updated to cite the new doc.
- [ ] BUILD_STATUS.md updated with PR number.

## Test plan

- No automated tests (docs-only).
- Manual: open the new doc + open `pricing/page.tsx` side-by-side and confirm every claim on the page appears in the doc.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- Authoring new substrate to satisfy 🔴 claims — those become follow-up briefs.
- Changing the pricing page UI or copy — operator-decisional.
- Adding the doc to the marketing-website build — separate brief.
- Localisation of the doc — not v1.

~150-300 LOC of markdown. Single doc touch + 4 small back-cite updates.
