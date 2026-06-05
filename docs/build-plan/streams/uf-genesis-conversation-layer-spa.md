# uf-genesis-conversation-layer-spa

**Owner**: claude.
**Status**: TODO 2026-06-06 (conductor split-out from `uf-genesis-conversation-layer`).
**Depends on**: `uf-genesis-conversation-layer-backend` (**#2724**, IN_REVIEW) — provides the `POST /api/v1/genesis/conversation` turn endpoint this consumes; A1 `/genesis` SPA (#2326) MERGED — provides the `CadenceExceptionsCapture.tsx` slot rendering this reuses. Governed by **[[DEC-039]]** (additive "Talk it through" mode, NOT a replacement of the one-shot flow).
**Authored against**: origin/main on `2026-06-06` (split from the parent brief at claim time per its SIZING note).
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-genesis-conversation-layer-spa`. Confirm the backend PR #2724 is MERGED first (the turn endpoint must exist on main).

## Why

The backend (#2724) shipped the multi-turn intake **engine + turn endpoint**: a
deterministic slot state machine, two bounded fail-safe LLM calls per turn, and
tenant-scoped persistence behind `POST /api/v1/genesis/conversation`. What's
missing is the **operator-facing surface**: a "Talk it through" entry beside
"Describe it" on `/genesis`, a chat UI, and a live "what we've captured" panel.

## What

Add the SPA half of the conversational intake:

- A **"Talk it through"** toggle/entry beside **"Describe it"** on `/genesis`
  (both land on the identical review → hire). Additive only — DO NOT remove or
  restructure the one-shot describe-and-watch flow (DEC-039).
- A **chat surface** that POSTs one turn at a time to
  `POST /api/v1/genesis/conversation` with `{conversation_id, message, confirm}`
  and renders the returned `assistant_message` + running `transcript`.
- A live **"what we've captured" panel** reusing `CadenceExceptionsCapture.tsx`'s
  slot rendering, driven by the response's `captured` block (charter / persona /
  responsibilities / cadence / exceptions / success_criteria).
- On `status: "ready"` show the assembled spec for confirmation; on confirm
  (`confirm: true` → `status: "done"`) hand the gathered slots to the EXISTING
  one-shot `/api/v1/genesis` flow so materialize → review → hire is byte-identical
  to "Describe it" (no parallel client-side materializer).

## Backend turn contract (already shipped in #2724)

`POST /api/v1/genesis/conversation`, request `{conversation_id: string (required),
message?: string, confirm?: bool}`; response `{conversation_id, status:
"asking"|"ready"|"done", slot?, assistant_message?, captured: {charter, persona,
responsibilities, cadence, exceptions, success_criteria}, spec?, transcript:
[{speaker: "assistant"|"operator", text}]}`. The opening turn sends no `message`.
The client generates a stable `conversation_id`.

## May-touch

- `web/components/skycot/genesis/` — a "Talk it through" toggle + a chat surface
  component + a captured panel reusing `CadenceExceptionsCapture.tsx`'s slot
  rendering. New components, each < 500 LOC.
- `web/lib/api/genesis.ts` — add the conversation-turn call(s) beside the existing
  intake calls (do NOT fork the existing one-shot calls).
- `web/app/genesis/` (or the page that hosts `/genesis`) — add the toggle entry.
- Co-located `*.test.tsx` for the new components.

## Acceptance criteria
- [ ] "Talk it through" toggle appears beside "Describe it"; the one-shot flow
      still works unchanged (additive per DEC-039) — regression test.
- [ ] The chat drives a full back-and-forth against the live turn endpoint and
      surfaces the captured slots panel live as they fill.
- [ ] On confirm, the gathered slots reach the SAME `/api/v1/genesis` materialize
      → review → hire path (no parallel client materializer).
- [ ] `pnpm vitest run` green on the touched specs (report counts); all touched
      files < 500 LOC.

## Out of scope
- Backend engine / turn endpoint / persistence — shipped in #2724.
- Voice intake → `mission-genesis-voice-input-via-argo-voice-io`.
- Guardrails / systems / SOP-doc capture panels → the `mission-genesis-*` Phase-2 tree.

## Parallel-surface check
- `rg -l 'intake|conversation' web/components/skycot/genesis/ web/lib/api/genesis.ts` —
  build on the A1 capture component + the new turn endpoint; no parallel capture family.
- Reuse `CadenceExceptionsCapture.tsx` slot rendering; do NOT fork it.

## Source
[[DEC-039]] (additive conversation mode) · `docs/skycot/plans/USEFULNESS-FLOW-RECONCILED-BRIEFS-2026-06-05.md` (brief A3) · parent `streams/uf-genesis-conversation-layer.md` (SIZING split) · backend #2724.
