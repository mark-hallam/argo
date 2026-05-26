# fix-onboarding-first-hire-populate-persona-bio-and-voice-preamble

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-first-hire-populate-persona-bio-and-voice-preamble`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F15 (P3) + cascade onto F11 (P2).

🟡 **P1 — When a worker is hired via the onboarding wizard, the persona row's `bio` field lands empty/NULL and `voice_preamble` is ~40 chars of boilerplate ("I'm a marketing-intern style worker named X"). The "describe what they should do this week" prompt the operator types in step 5 lands in the worker's `charter` but does NOT propagate to `persona.bio`. Worker-profile "About" section (which renders `persona.bio` when present) is therefore blank for every fresh hire — the personality dimension the wizard promised never materializes.**

## Root cause

`crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` (the first-hire handler) creates the worker row + persona row + charter row from the wizard's `description` field, but only the charter receives the freeform text. The persona insert uses templated values:

- `bio` → empty / NULL
- `voice_preamble` → `format!("I'm a {} style worker named {}.", vertical_id, name)` (~40 chars)
- `tone`, `signature`, `escalates_when` → default presets

The wizard's "describe what they should do this week" prompt is the richest signal the operator gives about who this worker should *be*. Routing only to charter means the worker-profile "About" section, the personality card's signature behaviors, and any persona-driven greeting in chat or email all fall back to generic content.

This compounds with F11 (`WorkerPersonalityCard.tsx:35-96` only has presets for Mariah/Priya/Jamal) — even if presets covered every name, the bio field underpinning the "About" section would still be empty for hires.

Two ways to fix:
1. **Mechanical** — copy the description string into `persona.bio` at insert time. Cheap; the bio reads as the operator wrote it.
2. **Genesis** — invoke the Genesis-plan stage (per `fix-onboarding-first-hire-invoke-genesis-plan`) which would synthesize a polished bio + voice_preamble + signature_behaviors from the description. Better content but blocked on Genesis end-to-end wire.

Recommend (1) for v1.0; (2) as a follow-up brief tracking Genesis adoption.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F15 [P3] — Persona `bio` empty + `voice_preamble` boilerplate for hire-flow workers
- `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F11 [P2] (partial — closing the bio gap lets the personality card consume real persona content; full closure requires `fix-worker-personality-and-level-derive-from-real-data`)

## Fix recipe

1. In `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` (first-hire insert path), set `persona.bio = description` (the full freeform text the operator typed).
2. Optionally tighten `voice_preamble` to use a slightly richer template that incorporates the verb-phrase from the description (e.g. `"I'm {name}, a {vertical_id} who drafts blog posts and runs weekly competitive scans."` — but this is dressing; the bio copy is the load-bearing fix).
3. Add an integration test in `crates/argo-cli/tests/` that POSTs to `/api/v1/onboarding/first-hire` with a known description and asserts the resulting `persona.bio` equals the description string.
4. Add a SPA-side fetcher path: ensure `web/lib/api/workers.ts` exposes `persona.bio` to the `WorkerOverview` shape; ensure `WorkerOverviewContent.tsx` "About" section renders `persona.bio` when present.

## Acceptance criteria

- [ ] `curl -X POST .../api/v1/onboarding/first-hire -d '{"description":"...","vertical_id":"..."}'` produces a persona row where `bio = description`.
- [ ] `/workers/<new-id>/overview` renders the typed description in the "About" section, not blank.
- [ ] Integration test asserts the persona.bio insertion round-trips through the API.
- [ ] No regression for the canonical 5-persona spine (Mariah/Peta/Priya/Jamal/Nori) which seed via `argo-demo-seed` and have their own curated bios.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` (~10 LOC — set bio = description)
- `crates/argo-cli/tests/onboarding_first_hire.rs` or similar (~30 LOC integration test)
- `web/lib/api/workers.ts` (verify `persona.bio` exposed)
- `web/components/skycot/worker/WorkerOverviewContent.tsx` (verify "About" section renders bio when present)

## Out of scope

- Invoking Genesis for richer bio generation (covered by `fix-onboarding-first-hire-invoke-genesis-plan`).
- The personality card's per-name preset dictionary (covered by `fix-worker-personality-and-level-derive-from-real-data` + `fix-worker-personality-coverage-extend-to-nori`).
- The `name_from_description` parser (covered by `fix-onboarding-first-hire-name-parser-prefers-named-pattern`).

## Effort

S — ~50 LOC including the test. ~1 hour. P1 because the "onboard them like a human" pillar from the welcome screen demands the persona dimension actually populate from the wizard's freeform field.
