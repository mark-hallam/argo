# fix-demo-seed-cli-for-real-roster

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-seed-cli-for-real-roster`.
**Source**: e2e-ux-audit Agent 2 findings F2 + F3 + Agent 3 finding F4 (`docs/skycot/ux-audits/2026-05-25-1100-scene-1-and-2-claim-1-and-2.md` + `docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`).

🔴 **P0 DEMO-GATE — gateway has Maya (1 worker) where the runbook needs 5 canon workers. Non-canon names leak through fixtures.**

## Root cause

Three coupled defects around the demo roster:

1. **Gateway seeds only "Maya · marketing-intern".** The v1.0 runbook calls out 5 workers (**Mariah / Peta / Priya / Jamal / Nori**) for the `/workers` trading-card scene. Currently `curl /api/v1/team` returns `total_count: 1` with name "Maya". The 5 named personas the runbook narrates do not exist in the gateway. Per `feedback_no_theater_in_demo` Option B (operator-locked 2026-05-24), the demo MUST be REAL — the seed CLI rewrite was prescribed but isn't running for this dev profile.

2. **SPA fallback persona list disagrees with itself.** `web/lib/sample-personas.ts:171-188`:
   - `SAMPLE_PERSONA_IDS = [Mariah, Priya, Jamal, Nori, Sam, Rio]` (drops Peta, adds Sam + Rio)
   - `DEMO_SEED_PERSONAS = [Mariah, Peta, Priya, Jamal, Nori]` (the canon)
   
   If anything pushes the demo onto the mock fallback path, Scene 1 names are wrong.

3. **Non-canon names "Rio" and "Sam" appear on workflow cards.** `web/lib/sample-workflows.ts:103,124,135` — `lastEditor: "Rio"` (×2) and `lastEditor: "Sam"` (×1) appear on the BY-line of three workflow cards (`qualify_lead`, `morning_pipeline_digest`, `clean_customer_list`). The scope brief explicitly tags this as a Scene 5 P0 vocabulary mismatch.

Canon roster per memory `project_pm_cohorts_peta_patrick` + the runbook: **Mariah, Maya, Peta, Priya, Jamal, Nori** (+ Patrick once shipped). Sam + Rio remain defined in the personas file for tests only — they must NOT appear in any user-visible fixture.

## Findings this brief closes

- Agent 2 F2 [P0] — Gateway demo data is "Maya · marketing-intern", not the runbook's 5 workers
- Agent 2 F3 [P0] — SPA fallback persona list does not match the runbook (drops Peta, includes Sam + Rio)
- Agent 3 F4 [P0] — Workflow cards show non-canon worker names "Rio" and "Sam" (`qualify_lead`, `morning_pipeline_digest`, `clean_customer_list`)
- Agent 3 F10 [P2] — `lastEditor` value should be guarded by a roster-check unit test (subsumed: ship the test in this brief)

## Fix recipe

1. **Run / ship the demo-seed CLI to populate the 5-worker roster** on the demo tenant. The persona definitions for Mariah/Peta/Priya/Jamal/Nori already live in `web/lib/sample-personas.ts` (`DEMO_SEED_PERSONAS`); the gateway-side seed must mirror them. Cross-reference the in-flight briefs `fix-pr-2284-rebase-demo-seed-roster-peta.md` and `fix-pr-2291-rebase-demo-seed-cli-against-current-main.md` — this brief may simply be unblocked once those land. **Worker: confirm the demo-seed CLI lands and runs on the current demo tenant. If those briefs subsume this fix, mark this brief STALE-FIXED at claim time.**

2. **Align SPA fallback to canon.** In `web/lib/sample-personas.ts:181-188`:
   ```ts
   export const SAMPLE_PERSONA_IDS = [
     MARIAH.id, PETA.id, PRIYA.id, JAMAL.id, NORI.id,
   ];
   ```
   (Drop Sam + Rio from the demo fallback. They stay defined in the file for tests but are not exported in `SAMPLE_PERSONA_IDS`.)

3. **Rename non-canon authors in `sample-workflows.ts`**:
   - `Rio` → `Peta` (for `qualify_lead` and `morning_pipeline_digest`)
   - `Sam` → `Nori` (for `clean_customer_list`)
   
   Trivial 3-line edit in `web/lib/sample-workflows.ts:103,124,135`.

4. **Roster-check unit test** at `web/lib/sample-workflows.test.ts`:
   ```ts
   const CANON_ROSTER = new Set(["Mariah", "Maya", "Peta", "Priya", "Jamal", "Nori", "Patrick", "SKYCOT", "system"]);
   it("every SAMPLE_WORKFLOW_SUMMARIES.lastEditor is canon roster", () => {
     for (const w of SAMPLE_WORKFLOW_SUMMARIES) expect(CANON_ROSTER.has(w.lastEditor)).toBe(true);
   });
   ```
   Add a similar guard for `SAMPLE_PERSONA_IDS` so a future drive-by edit can't reintroduce Sam/Rio.

## Acceptance criteria

- [ ] `curl /api/v1/team` on the demo tenant returns Mariah + Peta + Priya + Jamal + Nori (5 workers).
- [ ] `SAMPLE_PERSONA_IDS` in `web/lib/sample-personas.ts` matches `DEMO_SEED_PERSONAS` exactly.
- [ ] No occurrence of `lastEditor: "Rio"` or `lastEditor: "Sam"` in `web/lib/sample-workflows.ts`.
- [ ] Roster-check unit test added + passes.
- [ ] Component test (`Workflows.test.tsx`) snapshot does not contain "BY RIO" or "BY SAM".
- [ ] Recorded demo can name all 5 workers on the `/workers` page without leaking Sam/Rio.

## May-touch

- `web/lib/sample-personas.ts` (`SAMPLE_PERSONA_IDS` alignment)
- `web/lib/sample-workflows.ts` (3-line rename)
- `web/lib/sample-workflows.test.ts` (NEW guard test)
- `crates/argo-cli/src/cli/demo_seed/*.rs` (only if the in-flight seed-CLI briefs haven't landed by claim time)

## Out of scope

- TeammateCard / Personality maps name-keyed (separate brief: `fix-worker-personality-and-level-derive-from-real-data` — deletes those maps entirely)
- Authoring Peta's specific personality / trait values (subsumed by personality-from-pack brief)
- Audit row "unknown" worker rows (separate brief: `fix-audit-unknown-worker-and-hipaa-fixture-rows`)

## Effort

~20-60 LOC + 1 guard test (the heavy lifting is in the in-flight seed-CLI briefs). ~30-60 min if those briefs have landed; longer if this brief subsumes them.
