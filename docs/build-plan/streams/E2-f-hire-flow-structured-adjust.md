# Stream E2-f — Hire flow: Structured Adjust drawer (charter / duty / voice editor)

**Owner**: claude.
**Depends on**: E0 MERGED; E2-a..e MERGED.
**Authored against**: origin/main `67093531` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E2-f-hire-flow-structured-adjust`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E2 brief 6 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Direct edit vs. natural-language | Two paths surfaced side-by-side: free-text (E2-e) or structured editor (this brief). Operator picks. | Same | n/a |
| Voice profile mutation | Sliders + tone enum → updates `workerDraft.persona.voiceProfile` directly; no Genesis re-call | Server-side voice profile validation (legibility check, brand-safety) before persistence | `mission-enterprise-voice-profile-validation` |
| Duty editing | Add / remove / re-schedule duties via UI; client validates cron expressions | Server validates cron strings + checks for conflicts with existing duties | `mission-enterprise-duty-cron-validation` |
| Signature-phrase content | Free-text input with line-per-phrase; client filters obvious profanity via a small wordlist | Server-side moderation (LLM-classified harmful / brand-unsafe phrases blocked) | `mission-enterprise-voice-profile-moderation` |
| Persistence | Updates draft state only; commit happens on Hire button | Each Adjust action is its own audit row (so operator can trace what was changed when) | `mission-enterprise-hire-audit-bundle` (already named) |

v1 is acceptable because structured edits ONLY modify the
draft in client memory until the user hits Hire. Wrong edits
are recoverable via Cancel. The hire-commit moment is when
the server gets the full draft state — server can validate
+ reject + redirect to the editor at that point.

## Why

E2-e shipped the free-text Adjust drawer ("describe what
you'd change, Genesis re-runs"). That's the right path for
non-technical users + small tweaks. But power users + cases
where Genesis is wrong want **direct surgical editing** —
"toggle off this duty", "change tone to formal", "add
'Cheers!' as signature phrase". This brief delivers that:
a structured editor opened via a different entry point on
the Step 4 worker card.

Both paths exist; the free-text path stays for the
discovery / re-Genesis use case, the structured path is
for precise control.

## Scope

- Add a second entry point on `WorkerDraftCard` (from
  E2-e): an "Edit fields" link below the "Adjust" button.
  Click → opens the structured editor drawer.
- `StructuredAdjustDrawer` component (Radix Dialog from
  right):
  - **Charter** section: textarea pre-filled with the
    draft's charter, 5-row min.
  - **Duties** section:
    - List of duties from `workerDraft.duties[]`.
    - Each row: enabled toggle + label (editable
      inline) + schedule chip (editable inline).
    - `[+ Add duty]` button at the bottom.
    - Each row has a remove (×) button.
  - **Voice** section:
    - Tone: 3-button toggle group (warm / neutral /
      formal).
    - Formality: slider 0-100 (low → high).
    - Signature phrases: multi-line input, one phrase
      per line.
  - Footer: `[ Cancel ]` + `[ Save changes ]`.
- Save: writes the structured edits to
  `draft.workerDraft` (no re-Genesis call) + updates
  the example replies to a holding state
  `"Examples will refresh when you continue"` (since
  the cached examples no longer match the edited
  draft).
- Cancel: closes without changes.
- Validation:
  - At least 1 duty enabled (Save disabled otherwise).
  - Charter ≥ 20 chars.
  - Cron strings parsed client-side (use a tiny
    parser — see Notes).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/components/skycot/hire/
  WorkerDraftCard.tsx` (locks until merge), the
  `workerDraft` field of draft state.

## Files you may touch

- `web/components/skycot/hire/WorkerDraftCard.tsx` —
  narrowly scoped: add a second action link ("Edit
  fields") below the existing "Adjust" button. ≤ 280
  LOC after.
- `web/components/skycot/hire/StructuredAdjustDrawer.tsx`
  — NEW. The structured editor drawer. ≤ 450 LOC.
- `web/components/skycot/hire/DutyEditorRow.tsx` —
  NEW. One duty row in the editor (toggle + label
  + schedule). ≤ 200 LOC.
- `web/components/skycot/hire/VoiceProfileEditor.tsx`
  — NEW. Tone toggle + formality slider + signature
  phrases. ≤ 250 LOC.
- `web/lib/cron/parse.ts` — NEW. Tiny client-side cron
  parser/validator. Subset support: minute hour
  day-of-month month day-of-week. ≤ 200 LOC.
- `web/lib/cron/parse.test.ts` — NEW. ≥ 12 tests
  covering valid + invalid patterns. ≤ 200 LOC.
- `web/lib/moderation/phrase-wordlist.ts` — NEW. Small
  wordlist of obviously-blocked phrase strings.
  Loaded into a Set for O(1) lookup. ≤ 80 LOC.
- `web/lib/moderation/check-phrase.ts` — NEW.
  `isPhraseAcceptable(phrase): { ok: boolean,
  reason?: string }`. ≤ 60 LOC.
- `web/components/skycot/hire/StructuredAdjustDrawer.test.tsx`
  — NEW. ≥ 8 tests: drawer opens + closes, charter
  edit propagates, duty add/remove, tone toggle, cron
  validation, profanity-blocked phrase rejected,
  signature line filtered, save commits all fields.
  ≤ 350 LOC.

## Files you must NOT touch

- `web/app/(hire)/**` page files — locked by E2-a/e.
- `web/components/skycot/hire/Step4Introduce.tsx` —
  locked from E2-e (but the WorkerDraftCard subcomponent
  is narrowly extended as noted).
- `web/components/skycot/hire/AdjustDrawer.tsx` (the
  free-text drawer) — locked from E2-e. The two
  drawers are sibling components.
- `web/lib/api/genesis.ts` — locked. This brief does
  not call Genesis.
- `web/lib/hire/draft-store.ts` / `use-hire-draft.ts`
  — locked. State updates via existing `update()`
  method.
- Any non-hire surface.
- `crates/**` — sibling backend brief for cron
  validation + phrase moderation, not in this PR.

## Acceptance criteria

- [ ] WorkerDraftCard renders two action affordances:
      `[ ✎ Adjust ]` (from E2-e) and `[ Edit fields ]`
      (new). Both visible; semantically distinct.
- [ ] Tapping "Edit fields" opens
      StructuredAdjustDrawer.
- [ ] Drawer renders the three sections (Charter,
      Duties, Voice) per the wireframe.
- [ ] Charter section: textarea pre-filled from
      `workerDraft.charter`; ≥ 5 rows visible.
- [ ] Duties section:
      - Lists each duty as a DutyEditorRow.
      - Each row: toggle (enabled), label (inline-
        editable), schedule chip (inline-editable),
        remove (×).
      - `[+ Add duty]` adds an empty row at the bottom.
      - Removing a row removes it from state.
- [ ] Voice section:
      - Tone: 3 toggle buttons (warm/neutral/formal).
        Selected is filled-primary.
      - Formality: slider 0-100 with min/max labels
        (`"low"` / `"high"`).
      - Signature phrases: textarea, one phrase per
        line. Phrases ≥ 3 chars; profanity-list
        phrases removed on save with a warning toast.
- [ ] Validation rules:
      - At least 1 duty enabled: Save disabled
        otherwise.
      - Charter ≥ 20 chars: Save disabled otherwise.
      - Cron strings parsed: invalid cron renders
        red border + below-row error message; Save
        disabled.
- [ ] Save:
      - Writes all fields to `workerDraft` in draft
        state.
      - Closes drawer.
      - Step 4 re-renders.
      - Example replies show holding state:
        `"Examples will refresh when you continue."`
        (because the cached examples no longer match
        the edited draft).
- [ ] Cancel: closes without changes.
- [ ] Drawer is full-screen on mobile.
- [ ] 8 unit tests pass on the drawer; 12 on the cron
      parser.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 8KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E2-f-hire-flow-structured-adjust`
- PR title: `web(hire): structured Adjust drawer — charter / duty / voice editor (E2-f)`

## Notes for the implementer

- **Two-drawers semantics.** The free-text Adjust
  drawer (E2-e) is the *discovery / nuance* path —
  user describes a change in plain English, Genesis
  re-runs. The structured drawer (this brief) is the
  *precision* path — user edits specific fields
  without Genesis involvement.

  Don't try to merge them. Different user intents,
  different mental models, different cost profiles
  (Genesis re-run costs LLM tokens; structured edit
  is free).

- **Cron parser scope.** Keep it tiny — 5-field
  cron only:
  - Minute: 0-59 / `*` / `*/N` / `A,B,C`
  - Hour: 0-23 / `*` / `*/N` / `A,B,C`
  - Day-of-month: 1-31 / `*` / `*/N` / `A,B,C`
  - Month: 1-12 / `*` / `*/N` / `A,B,C`
  - Day-of-week: 0-6 / `*` / `*/N` / `A,B,C`

  No special tokens (`@daily`, etc.) — keep the
  surface minimal. Friendly preset chips inline on
  the schedule editor surface help users avoid raw
  cron:
  - `"Every morning"` → `0 6 * * *`
  - `"Every hour"` → `0 * * * *`
  - `"Weekdays 9am"` → `0 9 * * 1-5`
  - `"Once a week (Mon)"` → `0 9 * * 1`
  - Plus a `"Custom..."` option that lets the user
    enter raw cron.

- **Phrase wordlist.** Conservative starter list of
  ~40 obvious profanity tokens. Filtering is
  string-substring match (case-insensitive). When a
  signature phrase is rejected, show a toast:
  `"That phrase isn't allowed in your teammate's
  voice — try another."`. v1.5 swaps in LLM
  moderation.

- **Holding-state example replies.** When the user
  edits the draft via this drawer, the cached
  example replies (generated by Genesis in Step 3)
  no longer match. Don't re-run Genesis here (that's
  the free-text drawer's job). Instead show a
  holding state on the 3 example reply cards:
  ```
  +------------------------+
  | 💬  Examples will      |
  |     refresh when you   |
  |     continue.          |
  +------------------------+
  ```
  Visually muted. Doesn't block Hire (the actual
  generated examples will reflect the edited draft
  on the worker's first real action).

- **Mobile drawer.** Full-screen overlay on `< sm`
  (per the Track-D wireframe convention). Sections
  scroll vertically; footer (Cancel + Save) sticks
  to bottom.

- **Voice profile.** v1 ships 3 tone presets +
  formality slider + phrases. The voice profile
  schema already supports richer fields per
  `mission-voice-profile-richer-fields` MERGED;
  surfacing more fields (lexicon overrides, taboo
  words, signoff variants) is a v1.5 enhancement
  via the worker profile page's voice editor (see
  worker-profile wireframe).

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-voice-profile-validation` —
  server-side voice profile legibility + brand-
  safety check before persistence.
- `mission-enterprise-duty-cron-validation` —
  server-side cron parsing + conflict-detection
  against existing duties.
- `mission-enterprise-voice-profile-moderation` —
  LLM-classified harmful / brand-unsafe phrase
  blocker (replaces the wordlist).
