# Conductor advisory — Track E slices E2 / E3 / E4 / E5 (31 briefs)

> **STATUS: ACTIONED 2026-05-12** — rows inserted into BUILD_STATUS.md;
> E3 closed (5/5), E1 closed (6/6), E2/E4/E5 in flight. This
> advisory is retained for traceability only. Live state:
> BUILD_STATUS.md.

Authored 2026-05-11 against origin/main `3c4b72fe` after Track-E
author phase completed. Companion to the earlier
`conductor-advisory-E0-E1.md`. Paste the fenced block below into
the running conductor at the next tick. The conductor inserts 31
new TODO rows.

These briefs collectively implement the four remaining surfaces:

- **E2 hire flow** (9 briefs)
- **E3 team page** (5 briefs)
- **E4 worker profile** (9 briefs)
- **E5 audit timeline** (8 briefs)

All depend on **E0-frontend-foundation MERGED** as the
gating contract. Within each slice, sequencing is
identified per-row.

## Paste this to the conductor

```
MID-TICK ADVISORY — Track E slices E2/E3/E4/E5 briefs queued.
Thirty-one new TODO rows for the ledger. All depend on E0
MERGED; specific intra-slice dependencies noted per row.

Documents changed:
  - PRODUCT_ROADMAP.md (no status change — Track E remained
    IN PROGRESS through these authors)
  - 31 new files under docs/build-plan/streams/ (one per brief
    listed below)

Action required from you this tick:
  Insert thirty-one new TODO rows in BUILD_STATUS.md. Commit
  ONE row per commit per existing convention.

────────────────────────────────────────────────────────────
E2 — HIRE FLOW (9 briefs)
────────────────────────────────────────────────────────────

Row 1 — CONTRACT for E2 slice
  brief id:  E2-a-hire-flow-shell
  owner:     claude  status: TODO  PR: —
  depends:   E0 MERGED
  note:      "**Track E slice E2 (hire flow), brief 1/9
              CONTRACT.** Routes /hire + /hire/step/<N> +
              /hire/done/<id>. HireShell composite. Local-
              storage draft store with 7-day TTL. Step
              placeholders for E2-b..i. Brief:
              streams/E2-a-hire-flow-shell.md."

Row 2
  brief id:  E2-b-hire-flow-step1-land
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E2-a MERGED
  note:      "**Track E E2, brief 2/9.** Step 1 (Land) — free-
              text input + 5 example role tiles + Continue ≥
              15 chars. Brief:
              streams/E2-b-hire-flow-step1-land.md."

Row 3
  brief id:  E2-c-hire-flow-step2-listen
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E2-a/b MERGED
  note:      "**Track E E2, brief 3/9.** Step 2 (Listen) —
              Genesis-lite clarifying questions with 3-retry
              backoff + skip-to-template fallback. Brief:
              streams/E2-c-hire-flow-step2-listen.md."

Row 4
  brief id:  E2-d-hire-flow-step3-drafting
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E2-a/b/c MERGED
  note:      "**Track E E2, brief 4/9.** Step 3 (Drafting
              transition) — 5-stage cosmetic progress checklist
              + Genesis-lite worker-draft generation. Brief:
              streams/E2-d-hire-flow-step3-drafting.md."

Row 5
  brief id:  E2-e-hire-flow-step4-introduce
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E2-a/b/c/d MERGED
  note:      "**Track E E2, brief 5/9.** Step 4 (Introduce) —
              meet the teammate; inline-editable card + free-
              text Adjust drawer + Hire commit with optimistic
              advance. Brief:
              streams/E2-e-hire-flow-step4-introduce.md."

Row 6
  brief id:  E2-f-hire-flow-structured-adjust
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E2-a..e MERGED (E2-e specifically)
  note:      "**Track E E2, brief 6/9.** Structured Adjust
              drawer — direct edit of charter / duties / voice
              profile + client-side cron parser. Brief:
              streams/E2-f-hire-flow-structured-adjust.md."

Row 7
  brief id:  E2-g-hire-flow-step5-connect
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E2-a..e MERGED
  note:      "**Track E E2, brief 7/9.** Step 5 (Connect) —
              OAuth connector rows + computer-use session
              capture + channel preference selection. Brief:
              streams/E2-g-hire-flow-step5-connect.md."

Row 8
  brief id:  E2-h-hire-flow-celebration-banner
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E2-a..g MERGED
  note:      "**Track E E2, brief 8/9.** Step 6 celebration
              banner on approvals inbox with localStorage 24h
              TTL + active/inert variants. Brief:
              streams/E2-h-hire-flow-celebration-banner.md."

Row 9
  brief id:  E2-i-hire-flow-day1-wrap-duty
  owner:     claude  status: TODO  PR: —
  depends:   E2-e MERGED (hire commit endpoint exists)
  note:      "**Track E E2, brief 9/9 (BACKEND/Rust).** Day-1
              wrap duty — one-shot cron'd at hire+24h emitting
              activity summary via NotifyChannel. Brief:
              streams/E2-i-hire-flow-day1-wrap-duty.md."

────────────────────────────────────────────────────────────
E3 — TEAM PAGE (5 briefs)
────────────────────────────────────────────────────────────

Row 10
  brief id:  E3-a-team-page-shell
  owner:     claude  status: TODO  PR: —
  depends:   E0 MERGED
  note:      "**Track E slice E3 (team page), brief 1/5.**
              Page shell — route + sticky header + responsive
              grid + mock useTeam hook. Brief:
              streams/E3-a-team-page-shell.md."

Row 11
  brief id:  E3-b-team-teammate-card
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E3-a MERGED
  note:      "**Track E E3, brief 2/5.** TeammateCard composite
              with pause/resume context menu + 7-status pill
              support. Brief:
              streams/E3-b-team-teammate-card.md."

Row 12
  brief id:  E3-c-team-headline-generator
  owner:     claude  status: TODO  PR: —
  depends:   E3-a MERGED
  note:      "**Track E E3, brief 3/5 (BACKEND/Rust).** New
              argo-team-headline crate generating 2-3
              vertical-aware bullets + Tomorrow line per
              worker from audit + duty queries. Brief:
              streams/E3-c-team-headline-generator.md."

Row 13
  brief id:  E3-d-team-filter-search
  owner:     claude  status: TODO  PR: —
  depends:   E3-a + E3-b MERGED
  note:      "**Track E E3, brief 4/5.** Filter dropdown +
              search with URL state (?status=...&q=...) +
              FilterChipRail. Brief:
              streams/E3-d-team-filter-search.md."

Row 14
  brief id:  E3-e-team-empty-state-onboarding
  owner:     claude  status: TODO  PR: —
  depends:   E2-b MERGED (reuses EXAMPLE_ROLES); E3-a MERGED
  note:      "**Track E E3, brief 5/5.** First-hire empty state
              + 5 example role tiles + onboarding nudge.
              Brief:
              streams/E3-e-team-empty-state-onboarding.md."

────────────────────────────────────────────────────────────
E4 — WORKER PROFILE (9 briefs)
────────────────────────────────────────────────────────────

Row 15 — CONTRACT for E4 slice
  brief id:  E4-a-worker-profile-shell
  owner:     claude  status: TODO  PR: —
  depends:   E0 MERGED
  note:      "**Track E slice E4 (worker profile), brief 1/9
              CONTRACT.** Route /workers/<id>/<tab> + hero
              card + 5-tab strip + Worker type with org-
              hierarchy fields. Brief:
              streams/E4-a-worker-profile-shell.md."

Row 16
  brief id:  E4-b-worker-profile-overview-tab
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E4-a MERGED
  note:      "**Track E E4, brief 2/9.** Overview tab —
              WhatSheDoes (charter) + Duties + KPI block + 4
              sparklines + Recent activity. Brief:
              streams/E4-b-worker-profile-overview-tab.md."

Row 17
  brief id:  E4-c-worker-profile-activity-tab
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E4-a MERGED
  note:      "**Track E E4, brief 3/9.** Activity tab — worker-
              scoped audit feed with 3-dim filter; ships
              shared ActivityRow primitive consumed by E5-a.
              Brief:
              streams/E4-c-worker-profile-activity-tab.md."

Row 18
  brief id:  E4-d-worker-profile-memory-tab
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E4-a MERGED
  note:      "**Track E E4, brief 4/9.** Memory tab — Facts /
              Active conversations / Recent decisions / Voice
              profile summary with add/edit/delete. Brief:
              streams/E4-d-worker-profile-memory-tab.md."

Row 19
  brief id:  E4-e-worker-profile-skills-tab
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E4-a MERGED
  note:      "**Track E E4, brief 5/9.** Skills tab — Active /
              Library / Learned sub-sections with promote /
              try-it / disable / discard actions. Brief:
              streams/E4-e-worker-profile-skills-tab.md."

Row 20
  brief id:  E4-f-worker-profile-settings-tab
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E4-a MERGED
  note:      "**Track E E4, brief 6/9.** Settings tab — channels
              / quiet hours / budget / autonomy / advanced
              (operator-gated) / danger zone (delete with
              type-name-to-confirm). Brief:
              streams/E4-f-worker-profile-settings-tab.md."

Row 21
  brief id:  E4-g-worker-retrain-dialog
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E4-a..f MERGED
  note:      "**Track E E4, brief 7/9.** RetrainDialog — 6-step
              flow (Capture → Classify → Propose → Conflict
              → Regression → Commit) + RecentChangesPanel on
              Overview tab. Brief:
              streams/E4-g-worker-retrain-dialog.md."

Row 22
  brief id:  E4-h-worker-add-responsibility-dialog
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E4-a/b + E4-g MERGED
  note:      "**Track E E4, brief 8/9.** AddResponsibilityDialog
              — 6-step flow with 5 cron-preset chips + cron
              semantic preview. Brief:
              streams/E4-h-worker-add-responsibility-dialog.md."

Row 23
  brief id:  E4-i-kpi-sparkline-primitives
  owner:     claude  status: TODO  PR: —
  depends:   E4-b MERGED (initial inline sparkline exists)
  note:      "**Track E E4, brief 9/9.** Extract KpiSparkline
              to canonical home + add ProgressBar + DeltaChip
              + MiniBar primitives. Storybook stories. Brief:
              streams/E4-i-kpi-sparkline-primitives.md."

────────────────────────────────────────────────────────────
E5 — AUDIT TIMELINE (8 briefs)
────────────────────────────────────────────────────────────

Row 24 — CONTRACT for E5 slice
  brief id:  E5-a-audit-timeline-feed
  owner:     claude  status: TODO  PR: —
  depends:   E0 MERGED; E4-c MERGED (ActivityRow)
  note:      "**Track E slice E5 (audit timeline), brief 1/8
              CONTRACT.** Route /audit + cursor-paginated feed
              + day-group headers + search. Brief:
              streams/E5-a-audit-timeline-feed.md."

Row 25
  brief id:  E5-b-audit-filter-popover
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E5-a MERGED
  note:      "**Track E E5, brief 2/8.** 7-dimension filter
              popover (worker / channel / action type /
              status / approver / cost band / data
              classification) + chip rail. Brief:
              streams/E5-b-audit-filter-popover.md."

Row 26
  brief id:  E5-c-audit-action-drillin
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E5-a MERGED
  note:      "**Track E E5, brief 3/8.** Action drill-in —
              summary hero + inputs + decision chain (8 steps)
              + provenance + output with CU screenshot
              sequence. Brief:
              streams/E5-c-audit-action-drillin.md."

Row 27
  brief id:  E5-d-audit-replay-sandbox
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E5-c MERGED
  note:      "**Track E E5, brief 4/8.** Replay sandbox —
              verdict trichotomy (reproducible / drifted-
              aligned / drifted-misaligned) + output diff +
              5-axis provenance diff. Brief:
              streams/E5-d-audit-replay-sandbox.md."

Row 28
  brief id:  E5-e-audit-evidence-bundle-exporter
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E5-a + E5-c MERGED
  note:      "**Track E E5, brief 5/8.** Evidence bundle
              exporter dialog with 3 scopes × 4 formats
              (SOC2 / GDPR / HIPAA / Generic) + 3 size paths
              (≤50MB immediate / 50-100MB confirm / >100MB
              queued). Brief:
              streams/E5-e-audit-evidence-bundle-exporter.md."

Row 29
  brief id:  E5-f-audit-verifier-cli
  owner:     claude  status: TODO  PR: —
  depends:   E5-e MERGED (bundle format)
  note:      "**Track E E5, brief 6/8 (STANDALONE NPM).**
              @skycot/verify hash-chain verifier — offline,
              Ed25519-signed, MIT-licensed, separate verify-cli/
              package (not in web/). Brief:
              streams/E5-f-audit-verifier-cli.md."

Row 30
  brief id:  E5-g-audit-rbac-redaction
  owner:     claude  status: TODO  PR: —
  depends:   E0 + E1-e + E5-a/c MERGED
  note:      "**Track E E5, brief 7/8.** Cross-cutting RBAC
              redaction layer + ElevatedViewRequestDialog +
              RedactionBanner. Per-category permissions.
              Brief:
              streams/E5-g-audit-rbac-redaction.md."

Row 31
  brief id:  E5-h-audit-cross-system-jump-links
  owner:     claude  status: TODO  PR: —
  depends:   E5-c MERGED
  note:      "**Track E E5, brief 8/8 (LAST in Track E).**
              Cross-system deep-link generators for 8 SaaS
              (Stripe / Slack / GitHub / Linear / Salesforce /
              HubSpot / Zendesk / Jira) with strict origin
              allowlist. Brief:
              streams/E5-h-audit-cross-system-jump-links.md."

Commit each row insertion per existing convention, one
commit per row. Commit messages follow:
  status: new E2-a-hire-flow-shell TODO
  status: new E2-b-hire-flow-step1-land TODO
  ... etc through E5-h.

Worker pickup guidance:
  - **CONTRACT briefs first:** E2-a, E4-a, E5-a. Once each
    contract merges, downstream rows in that slice unblock.
  - **Parallel-safe** within a slice — multiple workers can
    claim rows in the same slice as long as dependencies
    are met.
  - **Backend briefs** (E2-i, E3-c) require Rust workers;
    others are TypeScript / Next.js.
  - **E5-f (verify-cli) ships an entirely separate npm
    package** outside the web/ tree; workers should review
    the standalone package layout in the brief.

Sibling backend briefs implied across the slices (NOT in
this batch — operator authors during Track F's backend
fill-in pass):
  - mission-gateway-genesis-classify-endpoint (E2-c)
  - mission-gateway-workers-hire-endpoint (E2-e)
  - mission-gateway-team-list-endpoint (E3-a)
  - mission-gateway-worker-detail-endpoint (E4-a)
  - mission-gateway-worker-overview-endpoint (E4-b)
  - mission-gateway-worker-activity-endpoint (E4-c)
  - mission-gateway-worker-memory-endpoints (E4-d)
  - mission-gateway-worker-settings-endpoints (E4-f)
  - mission-gateway-retrain-endpoints (E4-g)
  - mission-gateway-duty-endpoints (E4-h)
  - mission-gateway-audit-feed-endpoint (E5-a)
  - mission-gateway-audit-detail-endpoint (E5-c)
  - mission-gateway-elevated-view-request-endpoint (E5-g)
  - mission-compliance-hipaa-disclosure-log (E5-e)
  - mission-compliance-generic-export (E5-e)

Per the enterprise-grade-defaults memory, ALL briefs in
this batch state v1 posture explicitly and name Track-H
hardening briefs per shortcut. Total Track-H hardening
backlog identified across E2-E5: ~60 hardening briefs.
These run AFTER Track E base lands.

Resume your normal tick after the thirty-one row inserts
+ commits. Acknowledge by including the new row IDs in
your next status note.
```

## Operator-pending items

This advisory does NOT introduce new operator-judgment
questions beyond what's in each individual brief.
Slice-level open questions are documented in each brief's
"Open questions" section and flagged for the post-merge
operator review.

## What this advisory replaces

Nothing. This is the second conductor advisory for Track
E. The E0/E1 advisory remains valid and applies; this
one extends it with the remaining 31 rows.

## Next steps in the loop

After this advisory commits:
1. Update PRODUCT_ROADMAP.md Track E status note to
   reflect all 38 briefs authored.
2. Move to **Track F** — backend mission-claim-4 fill-
   ins. ~6 backend briefs from the existing
   MISSION_BACKLOG plus the sibling backend briefs
   listed above (~15 more).
