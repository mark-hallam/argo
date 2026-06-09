# DEC-042 — Continuous self-improvement loop (build loop + product loop)

- **Date:** 2026-06-09
- **Status:** **PROPOSED — awaiting operator sign-off before execution.** The operator chose "formalize the plan first" (2026-06-09); this ADR is the plan to review/approve. No build work starts until Mark approves the roadmap (and picks the Stage-1 start) below.
- **Owner:** conductor (Claude) — proposes; operator approves.
- **Relates to:** [DEC-029](DEC-029-truth-bounded-status-reporting.md) (NO THEATER / truth-bounded), [DEC-034](DEC-034-paying-ready-sprint-and-conductor-driven-run-mode.md) (conductor-driven run-mode, no cron), [DEC-041](DEC-041-execution-posture-risk-tiered-act-by-default.md) / [DEC-036](DEC-036-autonomous-scheduled-execution.md) / [DEC-037](DEC-037-graduated-real-execution.md) (execution posture), [DEC-017](DEC-017-teammate-eval-harness-synthetic-golden-companies-first.md) (eval harness), [DEC-025](DEC-025-genesis-done-right-full-synthesis.md) (Genesis), [DEC-002](DEC-002-macos-first-computer-use.md) (macOS-first computer-use).
- **Supersedes:** nothing. Extends the run-mode (DEC-034) with a durable self-improvement architecture.

---

## 1. Context

### 1.1 The operator's vision (2026-06-09, verbatim intent)
Build a **continuous, self-improving system** along two intertwined loops:

1. **The build loop** — Claude Code agents continuously, safely, and measurably improving the SKYCOT codebase, running at maximum useful autonomy.
2. **The product loop** — the platform continuously improving the **AI teammates** it gives users: teammates that do real-world work that is **measurable, traceable, and safe**, that **learn, use existing skills, build new skills, and have cross-platform computer use**, with everything they need to be genuinely useful and trustworthy.

> "The work must be measurable, traceable and safe." — operator

### 1.2 What the Boris Cherny research established (truth-bounded)
Research into how Boris Cherny (co-creator of Claude Code) runs loops/automations.
**Sourcing honesty:** YouTube blocked automated transcript extraction, so 1 of the 6 named videos was retrieved verbatim (Big Technology podcast, `Z6IT4gjrcPE`); the other 5 were confirmed by title/date and corroborated via his other interviews + community aggregation (howborisusesclaudecode.com, Pragmatic Engineer, WorkOS, Addy Osmani). Every pattern below appears in 3+ independent sources.

**Headline finding:** *his method already IS SKYCOT's architecture* — a controlling loop that dispatches parallel sub-agents which verify their own work and write their lessons back into a shared rules file. We are not pivoting; we are tightening. His three load-bearing principles each map to something SKYCOT already does:

| Boris principle | SKYCOT equivalent (already exists) |
|---|---|
| "Give the model a way to check its own work" → 2–3× quality (his #1 advice) | NO THEATER / truth-bounded status (DEC-029); eval harness (DEC-017) |
| A second model judges whether each tool call is safe (vs blanket skip-permissions) | Capability/approval fence; risk-tiered execution posture (DEC-041) |
| Every mistake becomes a committed rule, not a chat correction | CLAUDE.md + this decision register |

### 1.3 SKYCOT's actual current state (truth-bounded, verified this session)
- **Swarm:** `worker-supervisor` LaunchAgent + N worker bash loops (`claude-worker-loop.sh`) claiming the first eligible TODO row in `BUILD_STATUS.md`; the **conductor** (an interactive Claude session, per DEC-034) drives merges, replenishment, ledger-truth, and adversarial review. Cron conductor/evaluator stay DISABLED (DEC-034 — "I drive it").
- **Worktrees:** workers ALREADY isolate brief *code* in `/tmp/$BRIEF_ID` git worktrees. The residual race is the **shared `main` checkout** they `cd` back into to edit the `BUILD_STATUS.md` ledger, plus the conductor doing its own builds in that same checkout. (This bit us 2026-06-09: a gateway rebuild + workers collided; the swarm had to be paused to rebuild safely.)
- **Verification:** evaluator rounds + cold-cargo gates + the pre-merge gate exist, but a worker can still *declare* "done" before the gate runs — "done" is not yet mechanically bound to "gate passed."
- **Product primitives that exist:** hash-chained audit (traceable), approval gates across 9 channels (safe), eval harness (measurable), skills library + layered lookup + install/promote (use/share skills), Genesis (build a teammate), macOS computer-use executor.
- **Honest product gaps:** the learning loop's **skill-distillation step is a deterministic placeholder** (no real model pass yet — FEATURE_MATRIX Claim-4 row); **computer use is macOS-only** (Linux/Windows/E2B shipped but deferred-v1.x per DEC-002); measurability/traceability are backend-real but not yet first-class *product surfaces* a user sees.

---

## 2. Decision (proposed)

Adopt the **continuous self-improvement loop** as SKYCOT's standing operating architecture, made of two loops that share one safety spine:

- **Build loop** — the swarm + conductor, hardened with four upgrades drawn from the Boris research so that autonomy is *safe by construction*, not by trust.
- **Product loop** — the teammates' learn→build-skill→graduate cycle, with its two real gaps closed.
- **Safety spine (shared, non-negotiable):** every loop, in either layer, must end in a **checkable verifier** (a test pass, a health endpoint, a screenshot diff, a hash-chained audit row) — never a model's self-report. This is the operationalization of DEC-029 and Boris's #1 principle, and it is what makes maximum autonomy safe.

This decision is the *architecture*. The *roadmap* (§5) is staged and gated so each step is independently verifiable before the next begins.

---

## 3. The build loop — four upgrades (Boris-derived)

Ranked by leverage × safety × how independently verifiable they are.

**B-1. Eliminate the shared-checkout ledger race.**
*Problem:* workers isolate code in worktrees but contend on the shared `main` checkout for `BUILD_STATUS.md` edits + the conductor's builds → divergence flakes, clobbered edits, and the manual "pause the swarm to build" dance.
*Fix:* give the conductor its own dedicated worktree for builds; make ledger writes race-safe (per-worker status fragments the conductor merges, OR a file lock, OR move ledger mutations to conductor-only). Boris: per-agent worktrees are "the single biggest productivity unlock."
*Verifiable by:* run N workers + a conductor build concurrently; zero divergence flakes over a fixed window.

**B-2. A `Stop`-gate hook that makes "done" mean "verified".**
*Problem:* a worker can declare done before the gate runs; "done" is trust-based.
*Fix:* a Claude Code `Stop` hook that refuses session exit until the gate passes (`cargo test` + clippy + <500-LOC budget + ledger-in-band). Mechanically enforced, not trusted. This is NO THEATER (DEC-029) compiled into the tool.
*Verifiable by:* a worker that leaves a failing test cannot exit "done"; the hook blocks it.

**B-3. Automated rule-capture (the compounding part).**
*Problem:* mistakes become *chat* corrections or manual ledger sweeps; the error rate doesn't trend down on its own.
*Fix:* when a reviewer (human or agent) flags the same class of issue 3+ times, auto-convert it into a CI lint rule or a CLAUDE.md / DEC entry — the "write it down, don't re-prompt" reflex, automated. (Boris's exact habit; also his pre-AI "3–4 occurrences → lint rule" instinct.)
*Verifiable by:* a recurring review nit appears as a committed lint/rule within one cycle, and stops recurring.

**B-4. Structurally separate "build" from "verify" agents.**
*Problem:* the agent that wrote the code is biased when grading it.
*Fix:* make a separate verify-agent a *required* stage of every loop (you have evaluator rounds — formalize them as a non-optional phase, different context from the builder). Reduces self-grading bias.
*Verifiable by:* every merged change carries a verify-agent verdict from a different context than the author.

**Safety rail (applies to all of the above):** the most aggressive autonomy (overnight, skip-permissions) runs inside a **sandbox tier** (`/sandbox` / container with filesystem+network isolation), never directly on the host. The host keeps the allowlist + approval fence. (Boris: blanket skip-permissions only inside a sandbox.)

---

## 4. The product loop — two real gaps to close

The vision's requirements already map onto shipped primitives (audit = traceable, approval gates = safe, evals = measurable, skills library = use/share). Two gaps are genuinely open:

**P-1. Make the learning loop distil REAL new skills.**
Today the cluster→propose→A/B→promote tick runs, but the **distillation body is a deterministic placeholder** (no model pass). Replace it with a real model pass that synthesises a reusable skill from successful task traces, gated by the eval harness (a new skill must beat the floor before promotion). This is the "teammates build new skills" half of the vision, made real. (Bounds: a newly-distilled skill is draft-only at birth and graduates per DEC-041/037 — it does not gain real-execution rights automatically.)

**P-2. Take cross-platform computer use off the deferred list.**
DEC-002 made macOS the v1 path; Linux/Windows/E2B executors are shipped but unpolished. The vision explicitly wants **cross-platform** computer use. Promote at least one non-macOS executor (recommend E2B — cloud sandbox, also satisfies the safety rail) to first-class + verified, so teammates can do real computer-use work where the user's work actually lives. (This is a v1.x scope expansion of DEC-002; flagged for explicit operator approval since DEC-002 is `active`.)

Both gaps inherit the safety spine: measurable (eval-gated), traceable (audited), safe (approval-gated on sensitive verbs, draft-at-birth).

---

## 5. Staged roadmap (each stage independently verifiable)

> Sequencing rationale (first-principles): harden the loop *before* pointing more power at it. An unhardened loop that self-improves faster also self-*breaks* faster. So build-loop hardening (Stage 1) is the foundation; product self-improvement (Stage 2) rides on top; cross-platform + measurability surfaces (Stage 3) come last.

- **Stage 0 — Record + instrument (no behaviour change).** This DEC + a CLAUDE.md "self-improvement loop" section + the north-star metrics (§6) wired into the conductor heartbeat so we can *see* whether the loop is improving. Verifiable: metrics emit.
- **Stage 1 — Harden the build loop.** B-1 → B-2 → B-3 → B-4, in that order (each independently verifiable per §3). Default-OFF where a flag is involved; flip after the blast-radius check.
- **Stage 2 — Real product self-improvement.** P-1 (real skill distillation, eval-gated). Verifiable: a distilled skill beats the eval floor and gets installed, audited, draft-only.
- **Stage 3 — Reach + surfaces.** P-2 (cross-platform computer use) + make measurability/traceability **user-facing** surfaces (the audit + eval + skill-growth become a "your AI workforce's track record" product view). Verifiable: a non-macOS executor does a real task; the user-facing track-record surface renders real data.

Each stage ends with a truth-bounded report (DEC-029): VERIFIED / PARTIAL / BLOCKED, with evidence.

---

## 6. Measurability — how we know the loop is actually improving (north-star metrics)

The loop is only "self-improving" if these trend the right way. Tracked in the conductor heartbeat / a dashboard:

**Build loop:** merged-PRs/day; **% of "done" claims that pass cold re-verify** (the NO-THEATER metric — should approach 100%); mean-time-to-green-CI; drift-row count (steady-state 0); defect-escape rate (bugs that reach `main`); recurring-nit count (B-3 should drive this down).

**Product loop:** teammate task **success rate** (eval-graded); skills **created** + **adopted**/week (P-1 working); approval-gate **catch rate** (sensitive actions correctly held); audit-chain **integrity** (always intact); time-to-first-useful-action for a new hire.

If a metric doesn't move, that stage didn't work — we don't claim it did.

---

## 7. Risks + mitigations

| Risk | Mitigation |
|---|---|
| Autonomy amplifies a bad change across the codebase fast | Stage 1 *before* Stage 2; B-2 Stop-gate; sandbox tier; conductor adversarial review on the 3 risky surfaces (tenant/billing/perimeter, per DEC-034) |
| "Comprehension debt" — shipping faster than the operator understands (Addy Osmani's critique of loop-engineering) | Operator stays the verifier on load-bearing calls; truth-bounded stage reports; this register keeps decisions legible |
| Self-improvement loop optimizes a proxy metric, not real value | North-star metrics tie to *user* outcomes (task success, catch rate), not just throughput |
| Cross-platform computer use widens the attack/credential surface | E2B (cloud sandbox) first; draft-at-birth; approval-gated sensitive verbs; DEC-002 expansion is explicit, not silent |
| Config/tool autonomy edits break the operator's environment | Backups + validation before any config write (as done for the remote-control fix 2026-06-09) |

---

## 8. What is VERIFIED vs ASPIRATIONAL today (NO THEATER, DEC-029)

- ✅ **Verified now:** the swarm runs; workers use code worktrees; the eval harness, audit chain, approval gates, capability fence, macOS computer-use, and skills library all exist and are wired; the remote-control persistent lever is set (`remoteEnabled: true`, 2026-06-09).
- 🟡 **Partial:** verification exists but "done" isn't yet mechanically gated (B-2); rule-capture is manual (B-3); evaluator rounds exist but aren't a *required* loop stage (B-4).
- 🗓️ **Aspirational (this DEC's work):** the shared-checkout race fix (B-1); real skill distillation (P-1); cross-platform computer use as first-class (P-2); user-facing measurability surfaces (Stage 3).

This DEC does **not** claim the self-improving loop is "live" — it proposes the architecture to *make* it so, stage by stage, each gated on evidence.

---

## 9. Open questions for operator sign-off

1. **Approve the architecture + staged sequence** (§2–§5) as the standing direction? (Stage 1 build-loop hardening first is the recommendation.)
2. **DEC-002 expansion:** approve taking cross-platform computer use (P-2) off the deferred list as v1.x scope? (It's an explicit change to an `active` decision.) Recommend: yes, E2B first.
3. **Sandbox tier:** approve running the most aggressive autonomy inside a sandbox/container rather than on the host? Recommend: yes.
4. **Where to start** once approved — the Stage-1 order (B-1 → B-4) is recommended; confirm or re-rank.

On approval, this status flips `active`, a CLAUDE.md "self-improvement loop" section is added, and Stage 0 (metrics instrumentation) begins.
