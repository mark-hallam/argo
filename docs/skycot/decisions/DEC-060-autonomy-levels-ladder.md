# DEC-060 — Teammate autonomy as a user-selected level ladder (not a global on/off)

**Status:** active — design locked 2026-06-22; implementation staged.
**Operator-locked:** 2026-06-22 ("work out how to deploy autonomy, maybe how Claude Code deploys it with levels of it that a user selects a level for" → chose the level names "always ask, accept safe, auto, bypass" and defaults "Manual default · Conservative ceiling").
**Cross-links:** DEC-007 / DEC-029 (NO THEATER), DEC-041 (manifest-backed duty risk resolver), DEC-051 (irreducible human-approval floor), DEC-042 (every loop ends in a checkable verifier), DEC-059 (per-tenant scheduled autonomy default OFF — superseded in shape by this ladder).

## Context

Autonomy shipped (DEC-059 Wave 2, #3009) as a **single per-tenant boolean** gated behind a **global `ARGO_DUTY_AUTOFIRE` env flag that is OFF**. With the master flag off, the duty scheduler installs `ClosedGate` (deny-all) and **no duty fires for any tenant regardless of the per-tenant setting** (`boot_spawns_duty.rs:104`). A `/settings` on/off toggle on top of that would be **theater** — a switch that does nothing — which DEC-007/029 forbid.

Separately, the enforcement machinery for *graduated* autonomy already exists and is unused at the UI layer:
- `ClosedGate` (deny-all → everything waits for approval).
- `AutoApprovalDutyGate` with `AutoApprovalPolicy { enabled, max_usd_micros_per_action, daily_budget_usd_micros }`, per-worker resolved policy, per-worker spend tracking, and a **manifest-backed risk resolver** (DEC-041) that tiers each duty on its REAL risk.
- A live **per-teammate** route `PATCH /api/v1/workers/<id>/autonomy` already accepting `level: conservative | balanced | aggressive`, already audited (`worker.autonomy.changed`) — but **not wired to the gate**.
- The DEC-051 irreducible approval floor.
- The hash-chained audit (the verifier).

So the honest design is the one the operator named: **the Claude Code model** — a *level the user picks*, where each level does something **real and different**, enforced by machinery that already exists.

## Decision

Autonomy is a **per-teammate level** the workspace owner selects from a **4-rung ladder**, bounded by a **workspace ceiling** and a **permanent floor**. Each rung maps to an enforcement primitive that already exists — no new gate, approval engine, or audit.

| Rung | Backing store value | What the teammate does on its own | Enforced by |
|---|---|---|---|
| **0 · Always Ask** *(default)* | `none` / unset | Prepares + proposes; **every** action waits for approval. No auto-settle. | `ClosedGate` |
| **1 · Accept Safe** *(ceiling default)* | `conservative` | Fires scheduled duties; auto-settles only read-only / internal, low-risk, low-cost steps within a tight envelope; outbound/over-budget escalates. | `AutoApprovalDutyGate` + tight caps + low risk-tier only |
| **2 · Auto** | `balanced` | Auto-settles routine, **reversible** outbound to **known** contacts within a wider envelope; consequential/first-time escalates. | same gate, looser caps |
| **3 · Bypass** | `aggressive` | Skips the approval **queue** for everything the floor allows, within spend caps; owner supervises by exception + audit. | same gate, highest caps |

**Effective level = `min(teammate_level, workspace_ceiling)`.**

### The permanent floor — no rung can lift it (incl. Bypass)

Aligned with DEC-051. These ALWAYS route to a human, at **every** level including Bypass — "Bypass skips the queue, never the floor":
- Money leaving the business / paying / refunding
- A **first-time** external recipient
- Deleting or anything irreversible
- Granting access / changing permissions

First-principles rationale (operator-locked working style): an AI acting on a *business's* behalf with real money + external comms must never have a true no-guardrail bypass. Claude Code's bypass is a developer accepting risk on **their own** machine; a SKYCOT teammate's "bypass" acts on a **customer's** money and reputation. The ladder therefore tops out at "broad auto within guardrails," never "no guardrails." `Bypass` = skip the approval **queue**, not the **floor**.

### Defaults (operator-locked: "Manual default · Conservative ceiling")

- New teammate → **Always Ask** (nothing autonomous; identical to today's all-off behavior, so the rollout changes nothing until an owner opts in).
- Workspace ceiling → **Accept Safe** (owner must raise it to permit Auto / Bypass).

### The global flag is repurposed, not kept

`ARGO_DUTY_AUTOFIRE` stops being a master on/off and becomes the **workspace ceiling** (an admin setting, default Accept Safe). This kills the all-or-nothing failure mode: raising the ceiling *permits* levels up to it; it does NOT silently make every teammate autonomous — each teammate is still set individually.

### Verifier (DEC-042 spine)

Every auto-settle vs hold, every fire, and every level change is a row in the hash-chained audit; the spend counter is real and enforced; **Pause autonomy** works. No rung claims a capability it can't enforce → honest by construction.

## Implementation (staged — machinery exists, this is wiring + one new rung + UI)

1. **Level → gate** *(core, ~1 crate)*: duty scheduler reads the teammate's level instead of the global env flag. `none` → `ClosedGate`; `conservative|balanced|aggressive` → `AutoApprovalDutyGate` with that rung's envelope. Add `none` (Always Ask) as the default + new bottom rung.
2. **Workspace ceiling**: repurpose `ARGO_DUTY_AUTOFIRE` → a workspace-settings ceiling; enforce `min(teammate, ceiling)`.
3. **Envelope in PATCH**: extend the existing autonomy PATCH to carry `max_usd_per_action` + `daily_budget_usd` alongside the level.
4. **Selector UI**: the locked-settings-language selector (`docs/skycot/design-explorations/2026-06-22/autonomy-levels/autonomy-level-selector.html`) + a workspace-ceiling control.
5. **Audit + verifier**: already exist.

## Consequences

- Closes the DEC-059 "no `/settings` toggle UI yet" gap **without** shipping theater.
- Existing behavior unchanged at rollout (everyone defaults to Always Ask).
- The "conservative/balanced/aggressive" storage enum is reused (display-renamed to Always Ask / Accept Safe / Auto / Bypass) — no migration churn on the stored value.
- **Open confirmation:** the floor-on-Bypass call is the one place this design overrides the literal meaning of "bypass." If the operator wants a true no-floor mode, that is a *separate* serious decision requiring its own ADR + pushback — not assumed here.
