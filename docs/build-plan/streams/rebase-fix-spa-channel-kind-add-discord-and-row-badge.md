# rebase-fix-spa-channel-kind-add-discord-and-row-badge

**Owner**: claude.
**Authored against**: origin/main `a5df90c41` on `2026-05-29`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh rebase-fix-spa-channel-kind-add-discord-and-row-badge`.
**Supersedes**: PR #2521 (closed for re-author 2026-05-29) + original brief `streams/fix-spa-channel-kind-add-discord-and-row-badge.md`.

🟠 **P2 — Discord is referenced nowhere visible in the SPA despite shipping in the gateway; /audit rows render no channel badge. Re-implement against current main; PR #2521's 267-LOC union-widening touched 17 callsites and multiple have drifted since.**

## What changed on main since PR #2521 was authored

The original PR's load-bearing change was widening `ChannelKind` from `slack | email | sms | voice` to a 9-value union including `discord`. On main today:
- `web/lib/types/channels.ts` does NOT exist (the canonical home PR #2521 proposed)
- `web/components/ui/ChannelBadge.tsx` still declares the 4-value union inline (line 6: `export type ChannelKind = "slack" | "email" | "sms" | "voice"`)
- `web/components/skycot/ActivityRow.tsx` was touched by `df90fe1fe` (audit empty-state + Self-hash → Action hash rename — #2600)
- `web/lib/sample-approvals.ts` was touched by `94ff0bf7d` (Active reviewers KPI from distinct pending approvers — #2629)
- `web/components/skycot/audit/AuditDayGroup.tsx` was touched by `1a8e4e3c8` (render day-grouped feed as <table> — #2458)

The brief is fully load-bearing — Discord is invisible in the SPA — but the PR's 17-callsite union widening cannot be merged cleanly.

## What this brief MUST ship

Re-implement the original PR's full posture against current `origin/main` HEAD:

1. **Canonical type** at `web/lib/types/channels.ts` (NEW): export `ChannelKind = "slack" | "discord" | "email" | "sms" | "imessage" | "whatsapp" | "voice" | "push" | "web"`. Match the operator-facing catalog at `web/lib/notify-channels/registry.ts`. Re-export from `ChannelBadge.tsx` + `sample-approvals.ts` so existing callsites compile without churn.
2. **ChannelBadge learns Discord** + a `compact` icon-only mode for dense rows. Tint `#5865F2`, glyph `Hash` (no `Discord` glyph in lucide-react 0.479). Channel name → `title` + `aria-label` for assistive tech.
3. **ActivityRow gains optional `channel` prop**: renders the compact badge inline with the verb. `AuditDayGroup` threads `row.channel` through. (`AuditRow` already carries the field — re-verify on current main.)
4. **Per-channel record callsites** need `discord:` entries: `FilterPopover.tsx`, `FilterChipRail.tsx`, `worker/ChannelStripCard.tsx` (label + icon), `filters/audit.ts` `CHANNEL_LABELS`, `filters/approvals.ts` `CHANNEL_KINDS`, `hooks/use-approvals-live.ts` `KNOWN_CHANNELS`. `ApprovalCard.tsx` also has its own inline `channel` union — switch to the canonical type.
5. **Sample data**: 1 Discord pending approval in `sample-approvals.ts` (e.g. Peta's release-blocker thread) + 2 Discord audit rows in `sample-audit.ts` (one succeeded, one pending).
6. **Tests**:
   - `web/components/ui/__tests__/ChannelBadge.test.tsx` — Discord variant + compact-mode + parametric per-channel ("renders a glyph + a11y label for channel %s") covering all 9 values.
   - `web/components/skycot/ActivityRow.test.tsx` — 3 cases for the new `channel` prop (Discord, Slack, omitted).
   - `web/tests/e2e/audit-channel-badges-visible.spec.ts` — Playwright pin: at least 3 distinct channel badges visible on /audit, including `discord`.
   - `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx` — bump pending-count assertion 5 → 6 for the new Discord seed (verify current count first; may have drifted via #2629).

## Acceptance criteria

- [ ] `bash scripts/check-brief-freshness.sh rebase-fix-spa-channel-kind-add-discord-and-row-badge` passes.
- [ ] `web/lib/types/channels.ts` exists with the 9-value union.
- [ ] `web/components/ui/ChannelBadge.tsx` re-exports from canonical; renders Discord tile (Hash icon, #5865F2 tint) + supports `compact` prop.
- [ ] All `Record<ChannelKind, …>` callsites have a `discord:` entry — `tsc --noEmit` is clean.
- [ ] Sample data has at least 1 Discord approval + 2 Discord audit rows.
- [ ] `/audit` page renders inline channel badges on each row.
- [ ] Playwright spec at `web/tests/e2e/audit-channel-badges-visible.spec.ts` asserts ≥3 distinct channel badges including discord.
- [ ] `pnpm typecheck` + `pnpm lint` + targeted vitest all green.

## May-touch

- `web/lib/types/channels.ts` (NEW canonical)
- `web/components/ui/ChannelBadge.tsx` (re-export + Discord variant + compact mode)
- `web/components/ui/__tests__/ChannelBadge.test.tsx`
- `web/components/skycot/ActivityRow.tsx`
- `web/components/skycot/ActivityRow.test.tsx`
- `web/components/skycot/ApprovalCard.tsx` (canonical type)
- `web/components/skycot/FilterChipRail.tsx` + `FilterPopover.tsx`
- `web/components/skycot/audit/AuditDayGroup.tsx`
- `web/components/skycot/worker/ChannelStripCard.tsx`
- `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx` (count bump)
- `web/lib/filters/audit.ts` + `web/lib/filters/approvals.ts`
- `web/lib/hooks/use-approvals-live.ts`
- `web/lib/sample-approvals.ts` + `web/lib/sample-audit.ts`
- `web/tests/e2e/audit-channel-badges-visible.spec.ts` (NEW)

## Out of scope

- Adding new gateway-side channel support (Discord interaction signature verify already shipped via #2222).
- Replacing the Hash glyph with a real Discord SVG (deferred until lucide ships one or operator approves a custom SVG drop).
- Wiring Discord into the live `/api/v1/approvals` endpoint shape (this brief is SPA-side only; sample data covers the demo path).

## Effort

~267 LOC across canonical type + 6 callsite Record entries + sample data + tests. ~2 hours.

## Source

- Original brief: `streams/fix-spa-channel-kind-add-discord-and-row-badge.md`
- Closed PR: #2521 (closed 2026-05-29 as REAL-CONFLICT-NEEDS-REAUTHOR; 17-callsite type union widening across drift since 2026-05-27)
- UX audit: `docs/skycot/ux-audits/2026-05-27-0415/scene-6-claim-6-trust-and-approvals.md` F12 + F13
