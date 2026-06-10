# fix-genesis-conversation-persona-name-must-win-display-name

**Owner**: claude or codex (web-only, ~15 LOC — fast-mode suitable).
**Depends on**: none.
**Authored against**: origin/main `02837c942` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-genesis-conversation-persona-name-must-win-display-name

## Goal (DEC-048 rule 2 — captured-equals-built, persona slot)

When the operator explicitly names their teammate in the "Talk it through"
interview, the minted worker must carry THAT name — not a word scraped from
the charter.

## Why now (live e2e 2026-06-11)

Operator answered "What would you like to call this teammate?" with **Ivy**.
The minted worker displays as **"Chase"** — the first word of the charter
("Chase overdue invoices…") — with the absurd tell `Persona: Ivy` sitting in
the description blurb. A captured slot silently losing to a heuristic is the
DEC-048 P1 bug class (the operator answered a direct question and the answer
was dropped).

## Root cause (VERIFIED — SPA-side; the backend is already correct)

`gatherSlotsFromCaptured` (`web/components/skycot/genesis/ConversationIntake.tsx:60-80`)
flattens the captured persona into the composite `roleDescription` TEXT
(`parts.push("Persona: " + …)`) instead of passing it as a field. The one-shot
`GenesisRequest` **already has a `persona` field**
(`routes_genesis_stream.rs:85`), and `derive_display_name`
(`materialize_naming.rs:43-51`) resolves explicit-charter-name → persona-slug
→ archetype — so with no persona supplied, the charter-name extractor wins
and finds "Chase". Fix the hand-off, not the namer.

## Fix (~15 LOC, 2 files)

1. `ConversationGatheredSlots` (+`persona: string`) and
   `gatherSlotsFromCaptured`: return the captured persona as a FIELD; STOP
   embedding `"Persona: …"` into `roleDescription`
   (`ConversationIntake.tsx:36-48, 60-80`).
2. `handleConversationComplete` (`web/app/(app)/genesis/page.tsx:169-184`):
   pass `persona: gathered.persona` through `startStream` → `streamGenesis`
   body (confirm `genesis.ts` already threads a `persona` body key; add if
   missing — sibling-file rule, genesis.ts is grandfathered at >500).
3. Check the naming precedence end-to-end: with persona supplied, the minted
   `vertical.name`/display name is the persona ("Ivy"), and the charter-word
   heuristic only fires when persona is absent (one gateway pin if
   uncovered: `persona_for(Some("ivy"))` wins over charter extraction).

## May touch

- `web/components/skycot/genesis/ConversationIntake.tsx` (463 LOC) + its test
- `web/app/(app)/genesis/page.tsx` (497/500 LOC — ≤3 LOC delta, watch the ceiling)
- `web/lib/api/genesis.ts` persona body key ONLY if absent
- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream_tests.rs` (one
  precedence pin, only if not already covered)

## Acceptance / verifier

1. vitest: `gatherSlotsFromCaptured` returns `persona: "Ivy"`; composed
   `roleDescription` contains NO `"Persona:"` text.
2. Live/E2E-shaped test: talk-through capture with persona "Ivy" → minted
   worker display name "Ivy" (not the charter's first verb).
3. Describe-it path unchanged (no persona → existing heuristic unchanged;
   `genesis_hire_naming_maya_in_charter_yields_display_name_maya` stays green).
4. tsc + vitest green; `page.tsx` ≤ 500 LOC.

## Size / owner / priority

~15-30 LOC incl. tests. **🟠 P2 (DEC-048 silently-dropped-capture class;
display-only blast radius).** No collisions — no in-flight briefs/PRs on the
naming path (verified 2026-06-11).
