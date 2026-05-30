---
id: DEC-026
date: 2026-05-31
status: active
supersedes: none
---

# DEC-026 — Overnight autonomous Genesis-completion run authorized (P1→P7, "never stop till launch-ready")

## Decision

The operator (2026-05-31, going to sleep) authorized a **continuous autonomous overnight run** to complete *Genesis done right*: the remaining phases **P1 (materializer) → P3 → P4 → P5 → P6 → P7** per [[DEC-025]] + `docs/skycot/design/2026-05-30-genesis-done-right.md`, driven by the [[DEC-020]] auto-resume tick. Operator's words: *"Fire up P1 then P3–P7. While I sleep. Never stop till it's launch ready."*

- P0 (eval/safety hire-gate) + P2 (capability fence) are already DONE + adversarially + live-verified (`build_commit 05f9ea2`) — the run builds on that foundation and must not regress it.
- **Each phase follows the proven pattern:** implement (one cohesive build-agent) → **independent adversarial bypass-hunt** → fix → deploy (`restart-live-gateway.sh --force`) → **live-verify on :7400**. The adversarial step is mandatory — it caught a real demo-path hole in *both* P0 and the fence.
- NO THEATER ([[DEC-007]]): every safety gate must really work (verified, not tests-only); the morning report is honest DONE / PARTIAL / BLOCKED.

## Why

Genesis is the core differentiator ([[DEC-025]]); the safety foundation is in; the operator wants the from-scratch-synthesis + the moat built out continuously while away. "Launch-ready" = all P1–P7 verified, which is realistically **more than one night** — so the standard is *maximal real, verified progress*, not a dressed-up "done." An operator-input blocker is marked BLOCKED and the machine moves to the next phase rather than stalling.

## How (operative state)

- Auto-resume tick `com.skycot.buildout-resume` (30-min) **re-armed**: `~/.argo/buildout-stop` cleared, launchd agent loaded. The tick gates only on `~/.argo/buildout-stop` (confirmed) — independent of the dormant worker-supervisor swarm sentinels.
- Resume anchor: the `🌙 OVERNIGHT 2026-05-31` section at the top of `docs/skycot/overnight-build-out.md`.
- **Off-switch:** `touch ~/.argo/buildout-stop` (+ `launchctl unload …com.skycot.buildout-resume.plist`).

Related: [[DEC-020]] (autonomous overnight + auto-resume mechanism), [[DEC-023]] (continuous-autonomous operating model), [[DEC-025]] (Genesis done right — the P0→P7 plan + locked decisions), [[DEC-007]] (no theater).
