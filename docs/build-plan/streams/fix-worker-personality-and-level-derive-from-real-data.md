# fix-worker-personality-and-level-derive-from-real-data

**Owner**: claude.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-personality-and-level-derive-from-real-data`.
**Source**: e2e-ux-audit Agent 1 findings F10 + F11 + Agent 2 finding F4 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md` + `docs/skycot/ux-audits/2026-05-25-1100-scene-1-and-2-claim-1-and-2.md`).

🟡 **P1 — every newly-hired worker is the same person. LEVEL is a hash. Breaks "teammate, not tool" claim.**

## Root cause

Two coupled defects in the SPA personality + level rendering:

1. **`WorkerPersonalityCard` falls back to a generic DEFAULT_PERSONALITY for any worker not named Mariah/Priya/Jamal.** `web/components/skycot/worker/WorkerPersonalityCard.tsx:99-110, 117`:
   ```ts
   const personality = (worker && PERSONALITIES[worker.name]) ?? DEFAULT_PERSONALITY;
   ```
   Personalities are keyed by **name** and only Mariah/Priya/Jamal are populated. Maya (and every brand-new hire whose name isn't one of those three) gets:
   - traits "Reliable, Curious, Helpful"
   - greatAt "Working through the responsibilities you set / Asking when something is ambiguous / Surfacing patterns…"
   - identical greeting copy
   The investor-narrative claim "your AI teammate is a person, not a tool" fails — every hire IS the same person.
   Same defect in `web/components/skycot/TeammateCard.tsx:51-67` for the `TRAITS_BY_NAME` map.

2. **`TeammateCard` LEVEL is a hash of the worker name.** `web/components/skycot/TeammateCard.tsx:59-71`:
   ```ts
   const LEVEL_BY_NAME = { Mariah: 5, Priya: 9, Jamal: 12, Nori: 15, Sam: 7, Rio: 4 };
   function hashLevel(name) { /* hash mod 18 + 2 */ }
   ```
   Maya the brand-new marketing intern hashes to LEVEL 16 — visible front-and-center on the trading card. The level pretends to be earned but is computed from the name string. The trust-grade memory `project_workers_page_design_locked` describes L0–L5 cohort grades — wire THAT.

## Findings this brief closes

- Agent 1 F10 [P1] — newly-hired worker gets DEFAULT_PERSONALITY fallback, not a real persona
- Agent 1 F11 [P1] — `/team` card LEVEL is a hash of the worker's name, not real
- Agent 2 F4 [P0] — `TeammateCard` has no Peta entry in `TRAITS_BY_NAME` or `LEVEL_BY_NAME` (subsumed: rather than adding Peta's row, derive from real data)

## Fix recipe

1. **Personality from vertical pack + charter, not from name.**
   - The vertical pack manifest (loaded by the gateway at hire time) should carry default `traits[]`, `greatAt[]`, `escalateWhen[]`, `signatureBehaviors[]` for the role.
   - Surface these on the `/api/v1/workers/<id>/overview` response as a `personality: {...}` envelope.
   - SPA: remove the name-keyed `PERSONALITIES` map entirely. Read `overview.personality.*` directly. Fall back to a tasteful empty state ("Personality emerges as your teammate works") only when the vertical pack hasn't surfaced one yet — not to a generic fallback that pretends to be real.
2. **LEVEL from real trust grade, not from name hash.**
   - The trust-grade ratchet (memory `project_workers_page_design_locked`: L0–L5 cohort grades) computes from real action count + verified-decision count + escalation rate.
   - Gateway surfaces `trust_grade: "L0".."L5"` on `/api/v1/workers/<id>/overview` (likely already does — verify).
   - SPA: replace `LEVEL_BY_NAME` + `hashLevel` with `worker.trust_grade` directly. For a brand-new hire with 0 actions, render "L0" (or hide the level pill entirely until L1+).
3. **Delete the name-keyed maps.** They're load-bearing for the failure mode — leaving them in the codebase means the next refactor reintroduces the bug. Aggressive: rip out `LEVEL_BY_NAME`, `hashLevel`, `PERSONALITIES`, `DEFAULT_PERSONALITY`, `TRAITS_BY_NAME`.

## Acceptance criteria

- [ ] No name-keyed personality / level maps anywhere under `web/components/skycot/`.
- [ ] Hiring a fresh worker (any name) → `/workers/<id>/overview` renders a personality derived from the vertical pack OR a tasteful empty state — NOT a generic "Reliable, Curious, Helpful" fallback.
- [ ] `/team` `TeammateCard` LEVEL renders `worker.trust_grade` (L0..L5) or hides the pill — never `hashLevel(name)`.
- [ ] Visiting Maya (or any non-Mariah/Priya/Jamal worker) does NOT render the DEFAULT_PERSONALITY copy.
- [ ] Component test (`WorkerPersonalityCard.test.tsx`, `TeammateCard.test.tsx`) pinning the derivation paths.

## May-touch

- `web/components/skycot/TeammateCard.tsx` (LEVEL_BY_NAME, hashLevel, TRAITS_BY_NAME)
- `web/components/skycot/worker/WorkerPersonalityCard.tsx` (PERSONALITIES, DEFAULT_PERSONALITY)
- `web/lib/api/worker-overview.ts` (type: add `personality: {...}` + verify `trust_grade`)
- `crates/argo-cli/src/handlers/gateway/routes_workers_overview*.rs` (surface personality from vertical pack manifest)
- `crates/argo-vertical-pack/src/manifest.rs` (verify pack manifest already carries traits/greatAt/escalateWhen — surface them in the loader output)

## Out of scope

- Worker charter contract mismatch (separate brief: `fix-worker-detail-charter-object-vs-string-contract-mismatch`)
- Adding Peta + Patrick personalities specifically (subsumed: this brief deletes the name-keyed maps entirely, so the issue dissolves)
- Trust-grade ratchet implementation (assumed already live per memory `project_workers_page_design_locked`)

## Effort

~100-200 LOC across SPA + gateway. ~1-2 hours.
