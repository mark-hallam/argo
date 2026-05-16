# mission-v1-demo-recording-prep — Claude-doable prep for v1.0 demo recording (P0 SHIP GATE)

**Owner**: claude. **Depends on**: `claude-marketing-site-skycot-com` MERGED (#1754), `claude-email-template-system` MERGED (#1766), `mission-wire-browser-action-executor` MERGED (per ledger row 149).
**Authored against**: origin/main `c7275d4c` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-v1-demo-recording-prep`.
**Source**: ledger row 149 `mission-v1-demo-recording` notes: "Claude-doable prep portion (refresh `demo_script.md` for BrowserExecutor + Slack path; add a dry-run script via integration-test harness) is captured in a separate split brief `mission-v1-demo-recording-prep` (TODO if not yet authored)".

## Mission alignment

- **🎯 v1.0 SHIP GATE prerequisite** — operator cannot record `docs/skycot/demos/v1-mission-mvp.mp4` without an up-to-date script + validated flow.
- **Reliability** (CLAUDE.md priority #1) — the dry-run script catches breaks BEFORE the operator wastes a recording take.
- **Repeatability** (#3) — same flow runs identically every recording attempt.

## Scope

P0. Three deliverables that set up the operator to record:

### 1. Refresh `docs/skycot/demo_script.md` for current architecture

The current script is CLI-only (`argo workspace init`, `argo work add`, etc.) — pre-SPA, pre-marketing-site, pre-Stripe. Rewrite to match the v1.0 product surface:

- **Scene 0 (Setup, 0:00–0:30):** Operator has a fresh laptop, no SKYCOT installed. Show `skycot.com` landing in browser.
- **Scene 1 (Signup, 0:30–1:30):** Click "Hire your first worker" CTA → `/auth/signup` → enter email → magic-link email arrives → click link → `/onboarding/welcome`.
- **Scene 2 (Onboarding wizard, 1:30–3:30):** Step through 5-step wizard:
  - welcome → workspace name "Acme Co"
  - paste provider key (Anthropic)
  - pick vertical (marketing-intern)
  - first hire: "A marketing intern named Maya who drafts blog posts based on briefs"
  - lands on `/workers/maya`
- **Scene 3 (Worker takes action, 3:30–5:00):** Operator gives Maya a brief via chat. Maya proposes drafting a blog post. Auto-approval gate triggers ASK (low-trust new worker) — channel notify → Slack message.
- **Scene 4 (Slack approval round-trip, 5:00–6:00):** Operator approves in Slack with ✅. Action resumes. Maya posts draft to specified destination (Notion or just stdout for demo simplicity).
- **Scene 5 (Audit + close, 6:00–7:00):** Operator opens `/audit` in SPA, sees the action chain (TurnStart → ToolDispatch → ApprovalAsked → ApprovalReplied → ActionExecuted → TurnComplete). Hash-chain verifiability shown briefly. End.

Total: 5–7 minutes. Single take.

### 2. Dry-run script via integration-test harness

`scripts/demo-dry-run.sh` (NEW) — runs the validated demo flow against a local install + headless browser (via Playwright). Catches:

- Marketing CTA broken
- Signup endpoint 500
- Magic-link email format wrong / link 404
- Onboarding wizard step missing a button
- First-hire endpoint failing
- Approval ASK not firing
- Slack stub not receiving the ask
- Audit page not showing the chain

Output: `dry-run-results-YYYY-MM-DD.json` + screenshot of each scene's terminal state. Operator runs this script the night before recording; if green, recording can proceed.

### 3. Recording session checklist

`docs/skycot/demos/recording-checklist.md` (NEW) — practical playbook for the operator:

- Pre-recording: laptop battery, OS DND, screen resolution (1920×1080 vs 2560×1440), font sizes, browser zoom, terminal theme, mic test, dry-run pass.
- During recording: timer markers, scene boundaries, no key reveals, sanitized API keys (throwaway), sanitized email.
- Post-recording: trim to 5-7 min, save as `docs/skycot/demos/v1-mission-mvp.mp4`, sanitize check (no live tokens visible), upload to YouTube as unlisted for review.

LOC: ~300.

## Files you may touch

- `docs/skycot/demo_script.md` (existing — rewrite). ≤200 LOC.
- `scripts/demo-dry-run.sh` (NEW) — Playwright + curl + jq. ≤180 LOC.
- `scripts/demo-dry-run-fixtures/` (NEW directory) — test data (mock vertical pack, sanitized API key placeholder). ≤80 LOC across files.
- `docs/skycot/demos/recording-checklist.md` (NEW). ≤120 LOC.
- `docs/skycot/demos/README.md` (NEW) — index for demos directory; mentions `v1-mission-mvp.mp4` is operator-recorded artifact. ≤30 LOC.

## Files you must NOT touch

- Any backend Rust code — the demo script consumes existing surfaces, doesn't add new ones.
- The SPA — same.
- `mission-v1-demo-recording.md` brief itself — that's the parent deferred-operator brief; this is the prep split.

## Acceptance criteria

- [ ] `demo_script.md` updated to current architecture (SPA-first, no CLI scenes except optional dev-aside).
- [ ] All scene timings sum to 5-7 minutes.
- [ ] `scripts/demo-dry-run.sh` runs end-to-end against a fresh local install + Playwright headless.
- [ ] Dry-run script emits JSON results + per-scene screenshots.
- [ ] Dry-run script exits 0 on green, ≠0 on any broken scene with specific error line.
- [ ] Recording checklist covers pre/during/post including key safety + DND + dry-run gate.
- [ ] `docs/skycot/demos/README.md` exists as the parent doc for the recording artifact.
- [ ] No sensitive defaults in fixtures (real API keys, real emails).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-v1-demo-recording-prep`
- Title: `docs+test: refresh v1.0 demo script + dry-run harness (mission-v1-demo-recording-prep)`
- Worker verification: shellcheck + playwright dry-run + per SWARM_PROTOCOL.md.

## Hidden constraints

- **Recording is single-take.** The script must avoid mid-scene loading delays > 2 seconds — operator can't pause mid-take. Pre-warm caches in the dry-run.
- **Magic-link is real email, not console.log.** For recording, operator uses a throwaway address visible on-screen (e.g. `demo@throwaway.cc`). Dry-run uses a stub mail-catcher.
- **Slack workspace is real.** Operator needs to be signed in to a real Slack workspace before recording; the dry-run uses a stub webhook.
- **Provider key is throwaway.** Operator rotates the key after recording. Checklist enforces this.
- **No demo-specific code paths.** The recording exercises the SHIPPED product, not a demo-mode. Anti-pattern: don't add `if (demoMode) { skipApprovalGate }` — that's lying to viewers.
- **Browser-CU integration in scene 3** — optional. If Maya's first action is "draft a blog post → Notion", that doesn't need BrowserExecutor — Notion can be MCP. BrowserExecutor demo scene is a separate per-vertical script (`mission-per-vertical-demo-script.md` covers that). Don't conflate.

## Unblocks (post-merge)

- `mission-v1-demo-recording` (operator action) — operator can schedule recording session once dry-run is green.
- 2 BLOCKED `*-split` briefs (rows 173, 174) — operator-policy gate is "v1-demo-recording MERGED"; this prep is the substantive step toward that gate.
