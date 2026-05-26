# fix-approvals-sla-bar-rounding-60-minute-overflow

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-sla-bar-rounding-60-minute-overflow`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/scene-6-claim-6-trust-and-approvals.md` F2 [P1].

🟠 **P1 — SLA timer on `/approvals` renders the literal string "2h 60m left" once per hour for ~30 seconds. The rounding logic at `web/components/skycot/approvals/SLABar.tsx:46-50` rounds the residual minutes to 60 instead of carrying into the hours digit. A screen-recording camera will catch one if the recording rolls for more than a few minutes.**

## Root cause

`web/components/skycot/approvals/SLABar.tsx:46-50` (verify exact line range):

```ts
const hours = Math.floor(ms / HOUR);
const minutes = Math.round((ms - hours * HOUR) / 60_000);
return `${hours}h ${minutes}m left`;
```

When `ms` is e.g. `2h 59m 30s`:

- `hours = Math.floor(2h59m30s / 1h) = 2`
- `minutes = Math.round((2h59m30s - 2h) / 60s) = Math.round(59.5) = 60`
- Renders: `"2h 60m left"` instead of the intended `"3h left"`.

The bug is sub-frame-rate visible (only fires for the ~30 seconds before each whole-hour boundary), but a screen-recording over a multi-minute Scene 6 catches one across the dozens of visible SLA timers. Visibly amateur — the kind of glitch an investor's eye notices.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F2 [P1] — SLA timer renders impossible "2h 60m left"

## Fix recipe

1. **Replace `Math.round` with `Math.floor`** for the minutes computation:
   ```ts
   const hours = Math.floor(ms / HOUR);
   const minutes = Math.floor((ms - hours * HOUR) / 60_000);
   return `${hours}h ${minutes}m left`;
   ```
   This prevents the carry-into-60 case but introduces a different small bug: at e.g. `2h 59m 59s` the timer reads `"2h 59m left"` for the full last minute instead of `"3h left"`. Acceptable — humans round down.
2. **OR — round-with-carry**: compute total minutes, then split:
   ```ts
   const totalMinutes = Math.round(ms / 60_000);
   const hours = Math.floor(totalMinutes / 60);
   const minutes = totalMinutes - hours * 60;
   return `${hours}h ${minutes}m left`;
   ```
   This correctly renders `"3h 0m left"` at the boundary, then `"2h 59m left"` after. Slightly more code; truer to intent.
3. **Vitest unit test** at `web/components/skycot/approvals/SLABar.test.ts`:
   - 2h 59m 30s → "3h 0m left" (option 2) OR "2h 59m left" (option 1) — never "2h 60m left".
   - 0m 30s → "0m left".
   - 25h 5m → "25h 5m left".
   - Sweep boundary cases at exactly NhHM boundary.

## Acceptance criteria

- [ ] No input value to `SLABar` ever renders a minutes digit ≥ 60.
- [ ] The boundary at 2h 59m 30s renders sensibly (either as `"3h 0m left"` or `"2h 59m left"` — operator picks per Option 1 vs 2).
- [ ] Unit test covers boundary cases.
- [ ] Component test in `Approvals.test.tsx` (or the SLA dashboard test file) pins the rendered string for at least one fixture row.

## May-touch

- `web/components/skycot/approvals/SLABar.tsx:46-50` (the rounding logic)
- `web/components/skycot/approvals/SLABar.test.ts` (NEW or extend)
- Other consumers of the same time-formatting helper (grep for `Math.round.*60_000` across `web/components/`)

## Out of scope

- Re-design of the SLA visual surface
- Time-zone display (UTC vs Melbourne local) — separate concern
- SLA policy editing UI

## Effort

~10-20 LOC + 1 test file. ~30-45 min. P1 because investor-recorded camera time + visible glitch.
