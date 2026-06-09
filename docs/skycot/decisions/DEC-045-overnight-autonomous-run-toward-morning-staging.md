# DEC-045 — Overnight autonomous run 2026-06-09→10 toward morning gated-preview staging

- **Date:** 2026-06-09
- **Status:** **active** — operator-directed (2026-06-09, going to bed): *"go staging it in the morning, i am off to bed now. do autonomous working through the night."*
- **Owner:** conductor (Claude), self-paced via `/loop` ScheduleWakeup; the worker swarm builds 24/7.
- **Relates to:** [DEC-034](DEC-034-paying-ready-sprint-and-conductor-driven-run-mode.md) (run-mode), [DEC-044](DEC-044-verification-on-landing-and-ledger-source-of-truth-scope.md) (browser-verify gate + staged deploy), [DEC-032](DEC-032-deployment-readiness-phase.md) (deploy kit), [DEC-029](DEC-029-truth-bounded-status-reporting.md) (truth-bounded), [DEC-026](DEC-026-overnight-autonomous-genesis-completion.md)/[DEC-031](DEC-031-overnight-autonomous-frontier-b-buildout.md)/[DEC-033](DEC-033-overnight-build-toward-funding-meeting-thursday.md) (prior overnight-run pattern).

---

## Goal
Make tomorrow morning's **"stage the gated private preview"** a turnkey operation, and keep the build moving overnight — all truth-bounded (DEC-029), with an honest DONE/PARTIAL/BLOCKED report waiting when the operator wakes.

## Tracks
1. **STAGING PREP (primary).** Turnkey morning runbook (`docs/skycot/operator/MORNING-STAGING-RUNBOOK-2026-06-10.md`, written + pre-flighted by a deploy-prep agent: docker builds attempted, gateway bundling verified, seed CLI verified, operator-owed inputs enumerated). **Two real deploy-blockers found + fixed in-repo** this night: (a) `Dockerfile.web` missing `NEXT_PUBLIC_GATEWAY_URL` (prod prebuild guard requires HTTPS, non-localhost — `check-canonical-gateway-port.mjs:79-91`); (b) `Dockerfile.gateway` missing `libdbus-1-dev` (builder stage, for `libdbus-sys` via keyring). Verify both with real `docker build`.
2. **MERGE-TRAIN.** Gate + merge the PRs flowing from the 10 mission briefs. Web-only low-risk → DEC-006 lane (vitest green) + post-merge browser-verify. Rust → cargo gate (RAM now correctly measured via `scripts/free-ram-mb.sh`; gate in B-1's conductor build worktree). **Load-bearing (auth / tenant / billing / perimeter) → browser-verify or hold** (e.g. #2758 auth-middleware). **HELD for operator review: B-3 #2756 + B-4 (governance); #2751 needs a rebase.**
3. **QUEUE.** Keep fed; revive the Genesis cluster (re-authored against current main — never hand stale briefs) when low.
4. **BROWSER-VERIFY (DEC-044).** Verify landed web surfaces; auth-middleware verified before merge.
5. **MORNING REPORT.** Truth-bounded DONE/PARTIAL/BLOCKED + the staging runbook ready.

## Guardrails (non-negotiable)
- **NO THEATER (DEC-029):** verify before claiming done — build/test evidence in-hand; distinguish VERIFIED from APPLIED/EXPECTED.
- **Shared-checkout safety:** `git fetch` + rebase before push; explicit `git add`; **never** `git reset --hard` the shared tree.
- **Flags stay OFF:** `ARGO_DUTY_AUTOFIRE`, `ARGO_EXECUTION_GRADUATION`, `ARGO_STOP_GATE`.
- **Do not auto-merge governance** (B-3/B-4) or any unverified load-bearing change.
- **Memory:** RAM measured via the canonical helper (16 KB page size); pause the swarm only briefly if a launch-critical build needs headroom.
- **Recovery point:** git tag `checkpoint/dec042-approved-2026-06-09`.

## Verifiable / honesty
The deploy is "prepped + build-verified locally"; the **live stage is operator-owned** (AWS account + DNS + cert + secrets, per the runbook) and happens in the morning. This DEC does not claim the preview is "live" — it claims the kit is made turnkey overnight.
