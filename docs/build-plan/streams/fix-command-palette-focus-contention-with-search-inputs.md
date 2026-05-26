# fix-command-palette-focus-contention-with-search-inputs

**Owner**: claude or codex.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-command-palette-focus-contention-with-search-inputs`.
**Source**: ux-e2e-audit-T2309MEL `scene-4-and-5-claim-4-and-5.md` F12 [P3].

🟢 **P3 — Typing into the `/skills` hero search input occasionally causes the page to navigate away (to `/approvals`, `/hire`, `/team`) — destinations matching partial substrings of the typed query. Suspected ⌘K command palette grabbing global keystrokes when the input loses focus during a re-render. Intermittent + only seen via the Claude Preview MCP eval tool — may be a real bug OR a preview-MCP artifact. Operator should be aware during recording (avoid typing too fast in any search input).**

## Root cause (suspected)

The `<CommandPalette>` component (`web/components/skycot/CommandPalette.tsx`) likely listens to global keystrokes — at minimum the open-on-`Cmd+K` binding, possibly also a "start matching on any key while open" behavior. If the palette's open-state listener fires while a search input is focused AND the React tree re-renders mid-keystroke (causing the input to lose focus briefly), the next keystroke goes to the palette's command matcher instead of the input. The palette then matches the typed substring against `/approvals`, `/hire`, etc. and navigates.

Evidence from the audit:
- Reproducible only via the Claude Preview MCP eval tool (which dispatches synthetic keyboard events at machine speed)
- Destinations match partial substrings of the typed query (consistent with the palette's fuzzy-match → autonav behavior)
- Not localized; cannot reproduce deterministically

This could be a preview-MCP timing artifact (synthetic events bypassing focus debounce) — but the same pattern at low probability on real user input would still be a real bug.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F12 [P3] — Search input on `/skills` triggers what looks like global-hotkey contention with the ⌘K command palette

## Fix recipe

1. **Investigate first.** Before patching, confirm the bug is real on human keystrokes (not just synthetic events):
   - Open `/skills`, click into the hero search input
   - Type a 10-character query at normal speed
   - Repeat 20x; note whether focus ever escapes the input
   - Repeat with deliberately fast typing (auto-repeat held key)

2. **If reproducible — open the palette only on explicit chord.** `web/components/skycot/CommandPalette.tsx`:
   - Confirm the palette's keydown listener is gated on `event.metaKey && event.key === "k"` (or platform equivalent)
   - Ensure the listener does NOT auto-open on a stray "K" keystroke
   - If the palette is open, its keystrokes should be captured by its own input, not the page's global listener

3. **Defensive — palette should not navigate on auto-suggest match.** Per the audit's observation, the palette appears to be auto-navigating when the typed substring matches a route. Tighten:
   - Auto-suggest renders matches as clickable rows
   - Enter key navigates to the highlighted match
   - Typing alone should NEVER auto-navigate (no "navigate on first match")

4. **Stop event propagation in search inputs.** Defensive: hero search inputs (e.g. `/skills`, `/audit`) should `event.stopPropagation()` on keydown/keyup so the palette's global listener can't see them while the input is focused.

5. **Vitest spec.** `web/components/skycot/__tests__/CommandPalette.test.tsx`:
   - Type a query into a focused input
   - Assert the palette is NOT opened
   - Assert no navigation occurs

## Acceptance criteria

- [ ] Reproduction investigation documented in the PR description (real bug vs preview-MCP artifact)
- [ ] If real: typing into `/skills` hero search input never auto-navigates the page
- [ ] CommandPalette opens ONLY on the `Cmd+K` chord
- [ ] Hero search inputs stop keystroke propagation while focused
- [ ] Vitest spec passes
- [ ] No regression on the genuine `Cmd+K` palette open behavior (covered by existing palette tests if any)
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `web/components/skycot/CommandPalette.tsx` (~20 LOC keydown gating)
- `web/components/skycot/skills/SkillsSearchInput.tsx` (or equivalent — ~5 LOC stopPropagation)
- `web/components/skycot/audit/AuditSearchInput.tsx` (or equivalent — ~5 LOC stopPropagation)
- `web/components/skycot/__tests__/CommandPalette.test.tsx` (~80 LOC test)

## Out of scope

- Adding new keystroke chords / new palette behaviors
- Refactoring the search input components
- Other intermittent navigation issues (scene-1-and-2 F10 — separate bug, likely root-caused by `fix-worker-detail-charter-object-vs-string-contract-mismatch`)

## Effort

S. ~30 LOC + ~80 LOC test. ~1 hour. P3 because audit could not reproduce reliably — may be a preview-MCP artifact. Worth investigating but not load-bearing for the v1.0 demo if it doesn't reproduce on real human input.
