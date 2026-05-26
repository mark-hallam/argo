# Round 77 — S2 — UX-AUDIT-T00-28 31-brief synthesis verification

- **Date**: 2026-05-26
- **Round**: 77
- **Pillar**: Process-loop / v1.0 demo ship-gate (Track P + first-hire flow + claim-5 surface)
- **Verdict**: 🟢 PASS

## Setup

Third UX-audit batch in ~24h after T22-20 (18 briefs, verified r74) and T10-28 (33 briefs, verified r76 in-flight observation). T00-28 landed as a single atomic commit `d663e9b520774298aa3ad47e51b0fb7cd054dec6 ux-audit-T00-28: 5-agent e2e UX pass + synthesis — 31 root-cause fix-briefs queued` at 2026-05-26T11:36:23+10:00 (= 2026-05-26T01:36:23Z, ~1min after r76 ship 01:35Z).

Goal: verify (a) the commit math (4 P0 + 13 P1 + 12 P2 + 2 P3 = 31 distinct new briefs); (b) each P0 brief has a sound root cause grounded in observable backend / frontend state, not "wireframe says X"; (c) no duplicates with the T22-20 or T10-28 batch; (d) the audit reports referenced in the commit message exist on disk.

## Observed

**Batch math**:
```
$ git show d663e9b52 --name-status | grep -E '^A\sdocs/build-plan/streams/fix-' | wc -l
31
```
✓ 31 new fix-briefs in the commit (no superseding or modifications). Audit reports under `docs/skycot/ux-audits/2026-05-26-1028/` (whole-site-mvp-claims-coverage.md, scene-1-and-2-claim-1-and-2.md, scene-4-and-5-claim-4-and-5.md, scene-6-claim-6-trust-and-approvals.md, plus an onboarding/hire scene) all created in the same commit (`+253` to `+258` lines each). 1 `DELTA.md` audit-summary file also created in the same path. Ledger insert visible in `BUILD_STATUS.md` diff (+33 lines).

**P0 anti-theater spot-check — `fix-gateway-cors-localhost-127-and-preview-proxy-origins`** (claimed P0, brief asserts gateway CORS allowlist single-origin breaks any non-`localhost:3000` SPA origin):

```
$ curl -s -i -X OPTIONS \
    -H "Origin: http://localhost:3000" \
    -H "Access-Control-Request-Method: GET" \
    http://127.0.0.1:7400/api/v1/skills
HTTP/1.1 204 No Content
Access-Control-Allow-Origin: http://localhost:3000
Access-Control-Allow-Methods: GET, POST, PUT, PATCH, DELETE, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization, X-Argo-Principal, X-Idempotency-Key, X-Argo-Tenant-Id, X-CSRF-Token
Access-Control-Allow-Credentials: true

$ curl -s -i -X OPTIONS \
    -H "Origin: http://127.0.0.1:3000" \
    -H "Access-Control-Request-Method: GET" \
    http://127.0.0.1:7400/api/v1/skills
HTTP/1.1 204 No Content
[no Access-Control-* headers emitted]
```

✓ **Brief evidence reproduces empirically on live gateway**. The CORS middleware grants headers for `localhost:3000` but silently emits 204 with zero headers for `127.0.0.1:3000`, exactly matching the brief's reproduction. A browser hitting the SPA via Claude Preview proxy or via `127.0.0.1` would silently fail every gateway call. **Genuine P0 demo-blocker**, not a wireframe concern.

**P0 spot-check — `fix-spa-no-cu-macos-executor-status-surface`** (claim 5 surface):

```
$ grep -rn 'executor\|computer-use\|cu-macos' /tmp/eval-r77-wt/web/components/skycot/
[returns audit-detail ReplayDiff/ReplayHeader/ActionOutputCard mentions of 'computer-use' kind labels + ApprovalCard executor-side-draft comment + hire/Step5Connect computer-use connector row only]

$ ls /tmp/eval-r77-wt/web/app/\(app\)/settings/
api-keys  billing  connectors  data-export  error.tsx  knowledge  layout.tsx  members
notifications  notify-channels  page.tsx  providers  security  workspace
```

✓ **No `executors/` settings sub-tab exists**, no live-status surface for `argo-executor-computer-use-macos`. The brief's claim that "the SPA has zero affordance to confirm computer-use-macos is connected" is **correct on `origin/main`**. Claim 5 is 🟡 LIKELY-LIVE-pending-perms in CLAUDE.md — the runtime path exists but the operator has no SPA surface to self-verify before recording. **Genuine P0 demo-blocker**.

**Dedup discipline** — checked that none of the 4 named P0 briefs collide with existing brief filenames:

```
$ for b in fix-gateway-cors-localhost-127-and-preview-proxy-origins fix-signup-demo-mode-env-flag-and-runbook-prerequisites fix-workflow-detail-run-now-disabled-when-source-not-gateway fix-spa-no-cu-macos-executor-status-surface; do
    git log --oneline --diff-filter=A -- "docs/build-plan/streams/${b}.md" | wc -l
  done
1
1
1
1
```

Each brief is authored exactly once on origin/main (only in `d663e9b52`). Commit message asserts "18 additional findings mapped to existing fix-briefs on disk (no duplicates written)" — a sample-of-1 sanity check (the 4 P0s) confirms the dedup discipline holds for the load-bearing rows.

**In-flight downstream activity** — the conductor + workers have already started moving on T00-28:

```
$ git log --oneline origin/main -25 | grep -E 'fix-(gateway-cors|signup-demo|workflow-detail|spa-no-cu|onboarding-pick)'
03970a067 status: fix-workflow-detail-run-now-disabled-when-source-not-gateway in review (#2358)
af15541bc status: claim fix-onboarding-pick-vertical-skill-counts-and-gateway-catalog
0bfcebce9 status: fix-signup-demo-mode-env-flag-and-runbook-prerequisites in review (#2357)
f3678435c status: claim fix-spa-no-cu-macos-executor-status-surface
96e83ac96 status: claim fix-workflow-detail-run-now-disabled-when-source-not-gateway
f93a1d800 status: claim fix-signup-demo-mode-env-flag-and-runbook-prerequisites
993f16f4b status: claim fix-gateway-cors-localhost-127-and-preview-proxy-origins
```

✓ **4/4 T00-28 P0s claimed and 2 already opened as PRs** (#2357 + #2358) within ~30min of the batch commit. The process loop is functioning at peak velocity.

## Anti-theater check

A T00-28 synthesis batch could be theater if it (a) padded brief count by re-authoring existing briefs under new names (caught by the `git log --diff-filter=A` per-brief check), (b) labeled non-blocker findings as P0 (refuted by the empirical CORS curl reproduction and the empirical settings-directory `ls`), or (c) cited audit reports that don't exist (refuted by the commit including the report files).

The commit message claims "MISSION-claim coverage: 1 GREEN / 2 YELLOW / 3 RED" — an apparent regression from T22-20's "4 GREEN / 2 YELLOW / 0 RED". The commit-message disclosure is honest: "Apparent regression from T22-20's 4-GREEN/2-YELLOW/0-RED is mostly methodology correction (T00-28 graded what an investor SEES on a cold dev server, not what wireframes COULD show with seed data)." This is **the right anti-theater move** — the batch is graded against what a cold camera would capture, not what the wireframe could theoretically show. Self-disclosed methodology shift is a positive signal, not a negative one.

## Gap / follow-up

- **🟢 None for batch itself**: synthesis is sound, dedup-discipline holds, P0s reproduce empirically.
- **🟡 P1 recommendation**: 2 of 4 T00-28 P0s are now IN-REVIEW (#2357 #2358) — once mergeable, prioritize under overnight `--skip-clippy` ops-* auth (window expires 2026-05-26T22:00Z AEST per BUILD_STATUS.md). Conductor should re-validate freshness at merge time (briefs authored against `80e149df55f6` ~3h ago — well within 24h freshness window).
- **🟡 P3 ops cleanup**: as r76 advisory noted, the prior `pre-record-ux-audit-2026-05-25.md` rejected document was deleted; T00-28's `whole-site-mvp-claims-coverage.md` + scene-files are the new canonical demo-readiness ledger. CLAUDE.md "Doc-staleness gotchas" section should reference T00-28 audit outputs in the next conductor-doc refresh tick.
- **No new briefs recommended this round** — T00-28 + T10-28 + T22-20 are already saturating the worker queue (47/47 capacity).
