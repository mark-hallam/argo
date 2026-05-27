# fix-approvals-row-channel-chip-claim-6-visual-proof

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-approvals-row-channel-chip-claim-6-visual-proof`
**Source:** UX e2e audit 2026-05-28-0252 (scene-6 F3)

🔴 **P0 — `/approvals` data model carries `channel` on every approval row, but the dashboard renders no channel chip or icon anywhere. Scene 6 narrates "multi-channel approvals" with nothing on screen to back it. Claim 6's headline visual signal is invisible.**

## Root cause

`web/lib/sample-approvals.ts:17` defines `ChannelKind = "slack" | "imessage" | "sms" | "push" | "web" | "voice" | "whatsapp"` and every `ApprovalItem` in the fixtures + live envelope carries a populated `channel` field. The dashboard component `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` and its row children (`ApprovalRowActions.tsx`, KPI strip, severity-tier cards) never read or render that field — the only mention of `channel` in the dashboard file is the comment `// Undo back-channel.` (line 192).

Sibling brief `fix-spa-multi-channel-claim-6-proof-surface` (MERGED #2391) already adds a `<ChannelStripCard>` to the **worker overview** page that aggregates the last 7 days by channel. That is one Claim-6 proof beat but it does not cover the `/approvals` surface, which is where Scene 6 lingers. The data is already there per row — this is pure UI rendering.

## Findings this brief closes

- scene-6 F3 [P0] — `/approvals` UI never displays the channel field; Claim 6's "multi-channel" claim is invisible during Scene 6
- scene-6 F11 [P2] (partial) — operator can't tell from `/approvals` whether the click hit the real gateway or `mock-auth-bypass` (small data-source chip is the natural place to surface this alongside the channel chip; see Out of scope)

## Fix recipe

1. **Add a `<ChannelChip>` presentational component** at `web/components/skycot/approvals/ChannelChip.tsx`:
   - Props: `{ channel: ChannelKind, size?: "sm" | "md" }`.
   - Maps each `ChannelKind` to an icon (lucide `Slack` / `MessageCircle` for SMS+iMessage / `Phone` for voice / `Globe` for web / svgl-app brand SVG for Slack/Discord/WhatsApp from `web/public/logos/` per memory `feedback_logos_from_svgl_app`).
   - Maps each channel to a label ("Slack", "iMessage", "SMS", "Voice", "Web", "Push", "WhatsApp") with a small coloured dot using `--brand-token-*` from globals.css (no hex literals).
   - Renders as an inline-flex pill — icon + label, monospace `font-mono` is fine if Inter feels too heavy.
2. **Slot the chip on every approval card** at `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` row template:
   - Place it next to the worker name + risk tier on the left side of the row header, so it's visible at a glance.
   - Keep severity tier + `$ at risk` rail on the right unchanged.
3. **Add channel-count aggregates to the KPI strip OR a single inline "by Slack 3 · SMS 1 · voice 1" sub-line** under the "PENDING N" KPI tile. This makes the "we work across channels" claim a single-glance read.
4. **Storybook + vitest**:
   - `web/components/skycot/approvals/ChannelChip.stories.tsx` covering all 7 channels.
   - `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx` extending the existing test to assert at least one channel chip renders per row when the fixture includes a channel.
5. **Don't add new channels via this brief** — `fix-spa-channel-kind-add-discord-and-row-badge` (IN_REVIEW #2521) handles Discord. Once that lands the `ChannelKind` union widens and `ChannelChip` picks it up automatically via TypeScript exhaustiveness.

## Acceptance criteria

- [ ] Every approval row visible on `/approvals` renders a `<ChannelChip>` matching the row's `channel` field.
- [ ] Each of the 7 (or 8 post-Discord-merge) channel kinds maps to a distinct icon + label.
- [ ] `pnpm --dir web exec vitest run components/skycot/approvals/ApprovalsOpsDashboard.test.tsx` passes with the new channel-chip assertion.
- [ ] Storybook story renders all 7 kinds in light + dark mode without colour drift.
- [ ] Recording-day visual check: on `/approvals` the camera sees a Slack chip, an SMS chip, and at least one voice/iMessage chip across the visible cards (requires the demo seed to mix channels — that is a separate brief).
- [ ] No hex literals — all colours flow through `--brand-token-*` per `project_brand_consistency_pass`.
- [ ] PR body cites scene-6 F3.

## May-touch

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx`
- `web/components/skycot/approvals/ChannelChip.tsx` (NEW, ~60 LOC)
- `web/components/skycot/approvals/ChannelChip.stories.tsx` (NEW)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx` (extend)
- `web/public/logos/` — only if any brand SVG isn't already downloaded

## Out of scope

- Adding Discord to `ChannelKind` (covered by `fix-spa-channel-kind-add-discord-and-row-badge`)
- Mixing channels in the demo seed (covered by `fix-demo-seed-emit-worker-hired-audit-rows` once it ships)
- Data-source `mock-auth-bypass` chip on `/approvals` (defer to a small sibling — see scene-6 F12)
- The `<ChannelStrip>` on worker overview (covered by `fix-spa-multi-channel-claim-6-proof-surface` MERGED #2391)

## Effort

S — ~80 LOC new component + ~40 LOC dashboard wiring + ~60 LOC tests/stories. ~3-4 hours.
