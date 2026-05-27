# T0819MEL recovery handoff

If the Claude session that ran the T0819MEL ux-audit-4hourly pass died before
pushing, here's where things were and how to finish.

## What is durable on disk (safe)

All audit artefacts are written and survive session loss:

- 5 audit reports + DELTA.md + this RECOVERY.md at
  `docs/skycot/ux-audits/2026-05-27-0819/`
- 20 net-new fix-briefs at `docs/build-plan/streams/fix-*.md`
  (see "Briefs authored" list below)
- BUILD_STATUS.md modified: T0819MEL BATCH (20 rows + 1 header) inserted at
  line ~248, immediately above the T0739MEL batch
- SHARED.md heartbeat appended at end of file
- Synthesis manifest JSON at `/tmp/ux-audit-t0819-manifest.json`
- Insert script at `/tmp/insert-ux-audit-t0819-batch.py`

## What was in flight when the session paused

A single git commit (PID 51376 at pause) running the pre-commit hook (which
runs `lint-brief` against each of the 20 new briefs serially — each emits ~10
WARN-mode overlap notes but exits 0). 28 files staged. No push has happened.

## How to finish manually (operator-runnable)

```bash
cd ~/code/argo

# 1. Check if the original commit process is still alive. If yes, wait for it.
#    If no, the commit was either completed or interrupted — git status will tell.
ps aux | grep "git commit" | grep -v grep

# 2. Check current state.
git status -s | head -10
git log -1 --pretty='%h %s'

# 3a. If staged files are still listed (M/A) and no commit landed, run the
#     commit. The lint-brief hook is in WARN mode — all output is informational,
#     not blocking. Allow 5-10 min for the hook to grind through 20 briefs.
git commit -m "$(cat <<'EOF'
ux-audit-T0819MEL: 5-agent e2e pass + synthesis — 20 root-cause fix-briefs queued

5 parallel UX audit agents (onboarding hire-flow, scenes 1+2, scenes 4+5, scene
6+approvals, whole-site MVP) → synthesis clustered ~64 findings into 20 net-new
root-cause briefs + ~38 mapped to existing (no duplicates).

P0=4, P1=5, P2=5, P3=6. New P0s: useTeam stuck loading; audit redactor PHONE
false positive on action ids; SPA proxy audit detail 50x slower than gateway
direct; gateway first-hire second-call ghost worker.

MISSION-claim coverage from SPA: 0🟢 / 4🟡 / 1🔴 / 1❓. Closed in window: #2434
Claim-5 cold rerun (backend 🟢), #2436 fetchWorker storm + duty toggle leaks,
#2437 demo-seed per-tenant idempotency. Distance to v1.0 ship-gate: backend
5/6 🟢; SPA still 0/6 LIVE-from-camera; next gate is operator seed-CLI run.

Sources: docs/skycot/ux-audits/2026-05-27-0819/ (5 reports + DELTA.md).
EOF
)"

# 3b. If the commit already landed (git log shows ux-audit-T0819MEL at HEAD)
#     and `git status` is clean: just push.

# 4. Push.
git pull --ff-only origin main 2>&1 | tail -3
git push origin main

# 5. Verify.
git log -1 --pretty='%h %s'
```

## How to finish via a fresh Claude session

Paste this prompt into a new Claude session in `~/code/argo`:

> Read `docs/skycot/ux-audits/2026-05-27-0819/RECOVERY.md` and finish from
> where the prior session paused. Goal: commit the 28 staged files (T0819MEL
> ux-audit batch) and push to origin/main. Don't re-run any audit agents or
> the synthesis — all artefacts are already on disk.

## Briefs authored this pass (20 net-new, in the staged set)

P0:
- fix-gateway-first-hire-second-call-ghost-worker-not-persisted
- fix-useteam-query-stuck-loading-spa-fallback-never-resolves
- fix-audit-redactor-phone-false-positive-on-action-ids
- fix-spa-proxy-audit-detail-50x-slower-than-gateway-direct

P1:
- fix-onboarding-wizard-shell-demo-mode-banner-pass-through
- fix-app-shell-nav-hydration-mismatch-data-active-drift
- fix-worker-tab-strip-add-schedules-tab-vocab-consistency
- fix-workflow-viewer-builder-skeleton-only-ssr-no-generatemetadata
- fix-runbook-prewarm-add-audit-action-and-workflow-viewer-routes

P2:
- fix-gateway-verticals-api-v1-endpoint-empty-not-seeded-from-packs
- fix-onboarding-first-hire-add-explicit-name-field-honest-parse
- fix-settings-read-only-panels-no-onchange-no-state-wireup
- fix-team-page-hero-kpis-fabricated-from-worker-count-multiplication
- fix-demo-banner-dismiss-button-z-index-freeze-button-overlap

P3:
- fix-team-empty-state-gendered-language-her-to-them
- fix-vertical-packs-ship-zero-workflows-no-duties-after-install
- fix-workers-page-sr-only-h1-no-visible-page-heading-empty-state
- fix-onboarding-welcome-genesis-pillar-clickable-no-visual-affordance
- fix-audit-feed-table-semantic-html-not-tabular-screen-reader
- fix-approvals-kpi-em-dash-vs-zero-empty-state-vocab

## Top-3 highest-leverage briefs for the next worker to claim

(Per the DELTA.md synthesis — but note #2437 shipped during this audit window,
so the "162-min unfixed gateway-skills-boot-seed" thesis is partially obsolete;
the seed CLI now just needs to be RUN on the demo tenant for SPA Claim 4 to
flip 🔴 → 🟢):

1. Run the demo-seed CLI on the demo tenant (now that #2437 is in)
2. `fix-useteam-query-stuck-loading-spa-fallback-never-resolves` (Scene 1 nav-bar blocker)
3. `fix-spa-proxy-audit-detail-50x-slower-than-gateway-direct` (Scene 2 + 6 unblocker)
