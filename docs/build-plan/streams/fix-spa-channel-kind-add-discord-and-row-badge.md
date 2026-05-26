# fix-spa-channel-kind-add-discord-and-row-badge

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-channel-kind-add-discord-and-row-badge`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/scene-6-claim-6-trust-and-approvals.md` F12 [P2] + F13 [P3].

🟢 **P2 — Discord is referenced nowhere visible in the SPA despite shipping in the gateway (PR #2222 Discord interaction signature verify). The `ChannelKind` union types in `ChannelBadge.tsx` + `sample-approvals.ts` do NOT include `"discord"`. So a real Discord approval would render with the unknown-channel fallback. Separately, `ActivityRow.tsx` doesn't render a channel badge at all — channel info is only shown on the audit-detail page.**

## Root cause

### Part A — `ChannelKind` unions are missing Discord

`web/components/ui/ChannelBadge.tsx:6`:

```ts
export type ChannelKind = "slack" | "email" | "sms" | "voice";
```

`web/lib/sample-approvals.ts:17`:

```ts
export type ChannelKind = "slack" | "imessage" | "sms" | "push" | "web" | "voice" | "whatsapp";
```

Neither type includes `"discord"`. The gateway already accepts + emits Discord events (per PR #2222 Discord interaction signature verify), so the SPA's narrower union silently coerces any Discord channel value to the fallback badge.

The notify-channels registry at `web/lib/notify-channels/registry.ts:148` DOES list Discord (it's the connector-config UI's view). The inconsistency between the registry (Discord present) and the row-level ChannelKind unions (Discord absent) is the bug.

### Part B — `ActivityRow.tsx` doesn't render a channel badge

`web/components/skycot/ActivityRow.tsx` (verify) does NOT include the channel badge in the row layout. The audit detail page renders `<ActionSummaryHero>` which DOES include channel info, but the audit feed list-view is channel-blind. For Claim 6 "your AI teammates work across voice, SMS, Slack, Discord" the at-a-glance view should display channel.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F12 [P2] — Discord referenced nowhere visible in the SPA despite shipping in gateway
- scene-6-claim-6-trust-and-approvals F13 [P3] — Voice + SMS approvals exist in sample data but no channel badge on `/audit` rows

## Fix recipe

### Part A — extend ChannelKind unions

1. **Unify the two ChannelKind types** into a single canonical type at `web/lib/types/channels.ts` (NEW or canonicalize existing):
   ```ts
   export type ChannelKind =
     | "slack" | "discord" | "email" | "sms" | "imessage"
     | "whatsapp" | "voice" | "push" | "web";
   ```
2. **Re-export from both old locations** so existing imports continue to work.
3. **Update `ChannelBadge.tsx`** to render a Discord glyph (use the `lucide-react` MessageCircle or a Discord-specific SVG if licensing permits).
4. **Update sample-approvals + sample-audit** to include at least one Discord approval and one Discord audit row so the camera surface includes the badge.

### Part B — add channel badge to ActivityRow

1. **Read the channel from the row** (the existing audit row schema has a `channel` field per F13 in the audit report).
2. **Render a small `<ChannelBadge channel={row.channel} compact />`** in the ActivityRow's metadata column.
3. **Style**: 18px badge, no text label in compact mode — icon-only, channel name in `<title>` for screen readers.

### Part C — test coverage

- `web/components/ui/ChannelBadge.test.tsx`: render each ChannelKind variant; assert glyph + a11y label.
- `web/components/skycot/ActivityRow.test.tsx`: extend an existing test to include a `discord` channel row; assert the badge renders.
- Playwright `web/playwright/audit/channel-badges-visible.spec.ts`: navigate to `/audit`, assert ≥3 distinct channel badges visible across the visible rows.

## Acceptance criteria

- [ ] `ChannelKind` union includes `"discord"` and all 9 enumerated channels.
- [ ] `ChannelBadge` renders a Discord glyph for `channel="discord"`.
- [ ] `/audit` row list shows a channel badge per row.
- [ ] Sample data includes at least one Discord channel approval + one Discord audit row.
- [ ] All new tests pass cold.

## May-touch

- `web/lib/types/channels.ts` (NEW or canonicalize)
- `web/components/ui/ChannelBadge.tsx` (extend union + render Discord)
- `web/components/skycot/ActivityRow.tsx` (add badge)
- `web/lib/sample-approvals.ts` (add discord variants + update import)
- `web/lib/sample-audit.ts` (add discord rows + update import)
- `web/components/ui/ChannelBadge.test.tsx` (NEW or extend)
- `web/components/skycot/ActivityRow.test.tsx` (extend)
- `web/playwright/audit/channel-badges-visible.spec.ts` (NEW)

## Out of scope

- Multi-channel proof page (covered by `fix-spa-multi-channel-claim-6-proof-surface`)
- Discord OAuth integration UI (covered separately under connector-marketplace track)
- Voice-call playback affordance

## Effort

~150-250 LOC + 3 test files. ~4-6 hours. P2 because Claim 6 vocabulary completeness matters for the investor narrative.
