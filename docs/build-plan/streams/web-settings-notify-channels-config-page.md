# web-settings-notify-channels-config-page

> **Parallel-name note**: this brief is intentionally separate from `web-settings-connectors-page-build` (PR #2138, work-tool MCP-connector catalog at `/settings/connectors`). The two surfaces are disjoint: that page handles 29 *work-tool* connectors (Gmail/Linear/Stripe/etc.) used by the hire wizard; this page handles 11 *approval notification channels* (Slack/iMessage/Email/etc.) used by the dispatch_notify path. Both pages should coexist under `/settings/`. Lint warning about 2-token overlap is acknowledged.

**Owner**: claude.
**Depends on**: nothing (read-only operator config surface).
**Authored against**: origin/main `56015c0b9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-settings-notify-channels-config-page`.

🟡 **P2 — Operator UX: no SPA surface for configuring approval channels.** Source: `docs/skycot/evals/scheduled/2026-05-23/eval-channels-v1-config-vs-code-inventory.md` (Finding 3).

The audit established that the v1.0 SPA has no `/settings/connectors` (or any equivalent) page for the 11 approval channels. Operators today must read source to find env-var names, set them in `~/.argo/cred.toml` or `direnv`, and run `argo doctor channels` from a terminal. For a Pro-tier $29/seat customer this is workable but rough; for Enterprise it's a non-starter.

The `web/lib/connectors/registry.ts` file already exists and is the right shape — but its contents are work-tool MCP connectors (Gmail, Calendar, etc.) used by the hire wizard, not approval channels. This brief adds a sibling registry + page for the approval-channel surface.

## Scope

1. Add `web/lib/notify-channels/registry.ts` — analogous to `web/lib/connectors/registry.ts`, but listing the 11 approval channels with: id, display name, blurb, list of env vars to set, doctor-probe URL, tier-promise flag. Use `Mail`, `MessageSquare`, `MessageCircle`, `Phone`, `Smartphone`, `Inbox`, `Mic` icons from `lucide-react`.
2. Add `web/app/(app)/settings/notify-channels/page.tsx` — list page that fetches `/api/doctor/channels` (the existing `argo doctor channels` JSON output already wired through the gateway per `doctor_channels.rs`). For each channel row show: status pill (OK / MISSING / ERROR / SKIPPED), env-var checklist (set / unset), per-channel "show me the env vars" expandable, "Run probe" button that re-hits the gateway.
3. Add the row to `web/app/(app)/settings/page.tsx` settings nav so users can navigate to it.
4. **Do not add a "Connect" OAuth flow.** v1 ships env-var-only config; the page is read-mostly with copy-friendly env-var labels (`<code>SLACK_APPROVALS_BOT_TOKEN=...</code>`) the operator pastes into their shell. A later brief replaces the labels with real OAuth + secret-vault writes once `argo-credentials` ships per-tenant master keys (tracked separately).
5. **Pin the Pro-tier promise on the page** with a banner: "Pro tier promises Slack + iMessage + Email out of the box. Other channels are available if you set the env vars but are not in the tier card."

## Files you may touch

- `web/lib/notify-channels/registry.ts` — NEW (~250 LOC, 11 entries).
- `web/app/(app)/settings/notify-channels/page.tsx` — NEW (~300 LOC list page).
- `web/app/(app)/settings/page.tsx` — add the row + link.
- `web/__tests__/settings/notify-channels.test.tsx` — NEW (~100 LOC; render-and-snapshot per existing settings page test pattern).

## Files you must NOT touch

- `web/lib/connectors/registry.ts` — that's the work-tool MCP catalog; do not conflate.
- `crates/argo-channels/src/*.rs` — operator-config UX is web-only this round.
- `web/app/pricing/page.tsx` — tier promises stay as-is.
- Any OAuth flow code — out of scope per scope-item 4.

## Acceptance criteria

- [ ] Settings sidebar lists "Notification channels" between "API keys" and "Members".
- [ ] List page renders the 11 channels with: status pill, env-var copy-block, doctor-probe button.
- [ ] Pro-tier banner is present and accurate (Slack + iMessage + Email).
- [ ] Page is responsive (works at 375px / 768px / 1280px widths per the existing settings-page breakpoints).
- [ ] `pnpm test` clean for `web/__tests__/settings/notify-channels.test.tsx`.
- [ ] `pnpm lint --max-warnings=0` clean.
- [ ] No new Rust touched.

## Test plan

- Render-and-snapshot test asserting all 11 channels appear with the right env-var labels.
- Visual: open the page in dev mode, walk through every status state by mocking the doctor response.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- OAuth / Connect button flow — needs per-tenant credential storage substrate first (`mission-credentials-per-tenant-master-key` is the prerequisite, currently authored).
- Per-tenant routing of env-var names (today the gateway reads from process env; a multi-tenant gateway will need tenant-scoped credential lookup, which is a separate sweep).
- Adding the missing 4 channels (whatsapp/teams/matrix/imessage) to the dispatcher — owned by sibling brief `mission-imessage-wire-pro-tier-outbound-dispatch` for iMessage; whatsapp/teams/matrix stay DEFERRED-v1.x per operator decision.

~600 LOC across registry + page + nav + test. Web-only touch, no Rust.
