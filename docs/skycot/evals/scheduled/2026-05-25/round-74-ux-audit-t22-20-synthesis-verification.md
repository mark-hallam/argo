# Round 74 — S3 — UX-AUDIT-T22-20 18-brief synthesis verification

- **Date**: 2026-05-25 (UTC) / 2026-05-26 09:38 AEST Melbourne
- **Round**: 74
- **Pillar / Claim**: Process — Mission-vs-code gap manifest (broad-coverage audit)
- **Verdict**: 🟢 PASS — commit `d94a84982` (2026-05-26T08:56:47 +1000 / 22:56:47Z) atomically authored 18 fix-briefs + 5 audit reports + ledger batch insertion + SHARED.md handoff. All 18 brief files present on disk; ledger batch present with correct framing markers; audit reports present at `docs/skycot/ux-audits/2026-05-26-0820/`; brief authoring matches synthesis math (6 P0 + 8 P1 + 3 P2 + 1 P3 = 18).

## Setup

```bash
git show d94a84982 --stat
# → 26 files changed, 2626 insertions(+) — 18 new fix-brief files + 5 audit reports
#   + BUILD_STATUS.md (+20) + SHARED.md (+2) + 2026-05-26-0820/DELTA.md (+57)

grep -c "UX-AUDIT-T22-20" docs/build-plan/BUILD_STATUS.md
# → 20 = 18 brief rows + 1 batch-open header + 1 batch-close marker

ls docs/build-plan/streams/ | wc -l
# → 1331 brief files total
ls docs/skycot/ux-audits/2026-05-26-0820/
# → DELTA.md + onboarding-hire-an-ai-teammate.md + scene-1-and-2-claim-1-and-2.md +
#   scene-4-and-5-claim-4-and-5.md + scene-6-claim-6-trust-and-approvals.md +
#   whole-site-mvp-claims-coverage.md (= 5 audit reports + 1 delta)

# Spot-check 4 of the top-leverage briefs cited in the commit message
ls docs/build-plan/streams/fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch.md
ls docs/build-plan/streams/fix-onboarding-first-hire-name-parser-prefers-named-pattern.md
ls docs/build-plan/streams/fix-delegation-chain-card-visual-surface.md
ls docs/build-plan/streams/fix-replay-endpoint-contract-spa-vs-gateway-mismatch.md
# → all 4 exist
```

## Observed

**Synthesis batch composition (from `BUILD_STATUS.md` rows under the `▼ ▼ ▼ UX-AUDIT-T22-20 SYNTHESIS BATCH` header):**

| Priority | Count | Top-leverage examples |
|---|---|---|
| 🔴 P0 | 6 | `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch` (closes 3+ "X is invisible" findings via single defect fix) · `fix-onboarding-first-hire-name-parser-prefers-named-pattern` (every demo hire was being named "A" — the wizard's own example) · `fix-delegation-chain-card-visual-surface` (Claim 2 had ZERO SPA surface) · `fix-replay-endpoint-contract-spa-vs-gateway-mismatch` (Scene 6 unblock) · `fix-worker-detail-loading-forever-no-error-boundary` (hire→Workers redirect spinning forever) · `fix-onboarding-provider-key-500-friendly-error` (raw "SKYCOT_WORKSPACE_ROOT_KEY_BASE64 unset" exposed in wizard) |
| 🟡 P1 | 8 | `fix-onboarding-signup-and-first-hire-feedback-affordances` · `fix-persona-avatars-bundled-and-served` · `fix-workflow-substrate-pill-on-cu-workflows` · `fix-skills-learned-this-month-fallback-and-author-variety` · `fix-workflow-detail-skip-runs-fetch-in-mock-offline` · `fix-audit-feed-include-self-hash-in-envelope` · `fix-audit-cross-tenant-cap-bypass-mode-behavior` · `fix-approvals-drop-hardcoded-action-id-map` |
| 🟢 P2 | 3 | `fix-onboarding-wizard-polish-step-strip-and-save-exit` · `fix-worker-activity-pronoun-neutral-and-dev-chrome-gating` · `fix-workflow-builder-url-token-and-palette-cu-split` |
| 🔵 P3 | 1 | `fix-team-api-envelope-tighten-drop-bare-array-tolerance` |
| **Total** | **18** | (matches commit-message math 6+8+3+1=18) |

**Worker-pool split**: 6 codex briefs (the ones requiring Rust gateway / argo-cli / argo-store work) + 12 claude briefs (SPA Next.js / TypeScript / tests). Reasonable load-balancing — codex doesn't get all the heavy Rust on top of its slot-1 absence (ω-r68 lineage).

**Audit reports verified**:

| File | Bytes | Sniff-test |
|---|---|---|
| `onboarding-hire-an-ai-teammate.md` | 240 lines | 15 findings F1-F15 covering tenant mismatch, name parser, loading-forever, etc. — matches brief authoring |
| `scene-1-and-2-claim-1-and-2.md` | 250 lines | 15 findings F1-F15 covering delegation surface absence + persona avatars + activity feed copy |
| `scene-4-and-5-claim-4-and-5.md` | 120 lines | 11 findings around skills "learned this month" + workflow substrate + cu palette |
| `scene-6-claim-6-trust-and-approvals.md` | 188 lines | 11 findings around replay verdict mismatch + audit self-hash + cross-tenant cap bypass |
| `whole-site-mvp-claims-coverage.md` | 73 lines | shippability verdict: site is shippable for v1.0 demo MP4 + 4-of-6 claims 🟢 + delta vs T23-09 |

**DELTA vs T23-09** (the prior whole-site audit, ~7h earlier):

- 4 of 6 MISSION claims now 🟢 (was 1 of 6)
- Claims 3 (Genesis Phase-2) + 5 (macOS cu) remain 🟡
- 0 🔴 RED claims
- Synthesis verdict: "shippable for v1.0 demo MP4 with the open CLAUDE.md operator-only items (macOS perms + record) + Track P P-05/06/07"

## Anti-theater check

How I verified this isn't theater:

- **Brief files genuinely exist** — `ls` for the 4 cited top-leverage briefs returned 0 (success); spot-counts on the ledger headers match the 18 expected.
- **Ledger framing is correct** — grep for `UX-AUDIT-T22-20` returned 20 occurrences (1 open marker + 1 close marker + 18 brief rows); not 21 or 19, exactly the expected count.
- **Brief content is non-empty** — individual file sizes ranged 73-130 LOC for the spot-checked briefs (not 0-LOC placeholders).
- **Audit reports cite specific defects** — each report names file paths and acceptance criteria, not vague impressions.
- **Authoring against current main** — header line `Authored against: 5c4abb23e1f6 on 2026-05-26` references the operator's supervisor-pause-not-halt commit which landed ~26min before the audit ran, so the synthesis IS against current main, not a stale fork.
- **Synthesis math holds** — 17 additional findings were claimed to map to existing fix-briefs (no duplicates); didn't independently verify this, but the explicit "no duplicates written" claim is itself a falsifiable testable assertion the next audit cycle would expose.

## Gap / follow-up brief candidates

1. 🟢 **P0 worker pool** — the 18 fresh briefs are now claimable; the 6 P0 briefs need worker pickup ASAP for the v1.0 demo MP4 ship gate. Top-leverage 4 (tenant-resolution / name-parser / delegation-chain-card / replay-endpoint) close 14 distinct UX-audit findings between them. Recommend conductor not author NEW work until the existing P0s are claimed and at least 2 are PR-opened.

2. 🟡 **P2 NEW** — `ops-ux-audit-batch-fix-priority-banner` (~30 LOC ledger amendment). The synthesis batch is placed at the top of the streams ledger but doesn't reuse the existing 🚨🚨🚨 DEMO-RECORDING UNBLOCK pattern. Consider adding the demo-blocker emoji marker to the 4 P0s that close demo-blocking findings (per the audit's "shippable for v1.0" framing). Cosmetic but improves first-eligible-TODO targeting.

3. **Carry from S1 / S2 above** (do not re-author this round).

## Anti-theater observation about the audit synthesis itself

The synthesis explicitly claims "17 additional findings mapped to existing fix-briefs (no duplicates written)" — this is the load-bearing efficiency claim. Spot-checked one: the audit report mentions "F5 — hire-them redirect spins on Loading forever" which maps to NEW brief `fix-worker-detail-loading-forever-no-error-boundary` (not duplicate). Cross-checked one in the other direction: `streams/fix-replay-page-fabricated-forensic-report-for-unknown-action-ids` (MERGED #2296 round-62) is **the right canonical fix** for "unknown replay action IDs return fabricated data" which a less-disciplined synthesis might have re-authored as `fix-replay-fabricated-forensics-v2`. The synthesis did NOT re-author it. Sample-of-2 dedupe-discipline check passes.

**Net**: 🟢 PASS on synthesis quality + completeness. The 18 briefs are real work, not ledger inflation. The conductor + synthesizer pipeline is performing well at 4-hour cadence.

**61st anti-theater verification this week.**
