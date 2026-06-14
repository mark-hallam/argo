# Live Windows computer-use run (`win-e2e-computer-use-run-loop-live`)

**Question (DEC-054 capstone):** does a teammate actually drive THIS Windows
desktop — screenshot → real Anthropic model → real Windows input → verified
effect — with the DEC-051 ACT floor holding high-risk actions for approval?

**Verdict: ⚠️ HONEST GAP — the live model-in-loop run was NOT performed.** Both
operator-only preconditions are unmet on this autonomous, shared-session host, so
per the brief's explicit stop conditions the run stops here rather than fake a
pass. **The #2937 run-loop wiring is verified READY** (code-grounded below), and
an exact operator runbook to complete the live run is provided. NO THEATER: no
model was called, no input was fired, no screenshot was staged — this is a
precisely-located stopping point, not a fabricated green.

---

## Provenance

| Field | Value |
|---|---|
| Host | `Latitude5440` — Windows 11 Pro 10.0.26200, x86_64 |
| Binary | `argo 0.0.1 (f38280b 2026-06-14)` — fresh build, **0 commits behind `origin/main`** (the #2937 path) |
| UTC | 2026-06-14T06:46Z |
| `ANTHROPIC_API_KEY` | **ABSENT** (`[ -n "$ANTHROPIC_API_KEY" ]` → false) — no key to log even first-4-chars; precondition 1 fails |
| Session | **shared autonomous fleet session** (this Claude Code worker + a sibling worker on the same machine) — not a dedicated foreground desktop; precondition 2 fails |

---

## Readiness — the #2937 path IS wired (verified on current main)

The live CU run-loop is fully in place; nothing in the CODE blocks the run:

| Component | Evidence (file:line on `f38280b`) |
|---|---|
| `run()` orchestrator | `crates/argo-executor-computer-use-windows/src/executor.rs:186` — `pub async fn run(&self, req: WindowsCuRunRequest) -> WindowsCuResult<WindowsCuOutcome>` |
| Shared loop seam | `crates/argo-executor-computer-use-windows/src/executor_loop.rs:255` — `impl CuStepRunner for WindowsCuExecutor` |
| DEC-051 ACT floor | `crates/argo-executor-computer-use-windows/src/cu_risk.rs` — `CuRiskTier::{Observe, Act}` + `classify_kind` |
| Wired dispatch arm | `crates/argo-cli/src/handlers/worker_actions_executor_cu_windows.rs:115` builds `WindowsCuRunRequest`, `:203` calls the **real** `exec.run(req)` (no longer the preflight+HumanHandoff stub) |

So the loop will run the moment the two operator gates are satisfied. #2937
proved this HERMETICALLY (FakeRunner, no key/desktop); the only missing thing is
a keyed run on a dedicated desktop — which is exactly what is gated below.

---

## The two unmet preconditions (precise gap location)

### Precondition 1 — no `ANTHROPIC_API_KEY` on the host ❌

`run()` (executor.rs:186) builds a **direct Anthropic client from
`req.anthropic_api_key`** to drive the screenshot→model loop. There is no key in
the host environment (confirmed above), so the model turn cannot fire.

> NB: the `claude-code-cli` *provider* trick that let the **Genesis safety judge**
> run without an API key (see `E2E-FULL-JOURNEY.md`) does **NOT** apply here — that
> path goes through the provider-manifest system (`ARGO_MANIFESTS_DIR`), whereas
> the CU `run()` constructs a direct `AnthropicClient` from the key string. So
> the CU loop genuinely needs a real `ANTHROPIC_API_KEY`. (A possible future
> enhancement — routing the CU loop's model turns through the provider system so
> `claude-code-cli` could drive it keyless — is noted as a follow-up idea, not a
> claim.)

### Precondition 2 — no dedicated foreground desktop; firing input is unsafe here ❌

The loop fires synthetic `SendInput` into **whatever window is focused**. This
host is running the autonomous Windows fleet (this worker + a sibling worker in
the shared working tree). Firing real keyboard/mouse input would type into the
live session — disrupting the running fleet — which the fleet's own operational
rule explicitly forbids ("don't run input-firing smoke on the live session").
There is also **no operator present** to approve the DEC-051 ACT-floor holds:
the very first action (app-launch / `key` press to open Notepad) classifies as
**ACT** and HALTS for approval; with no approver the run would terminate at
`HumanHandoff` before completing the task anyway (and the autonomous `type`
staged before that hold would still fire real keystrokes into the focused
window). So a safe, complete autonomous run is not possible on this session.

This is the same operator gate that keeps the sibling row
`win-cu-executor-end-to-end-run` **BLOCKED**.

---

## Operator runbook — how to complete this live run

On `Latitude5440`, at a **dedicated, unlocked, foreground desktop** (close other
apps so input lands in Notepad), with a **real key**, and an operator present to
approve the ACT-floor holds:

```powershell
# 1. Build / confirm the binary is the #2937 path:
cargo build -p argo-cli ; argo --version    # expect a SHA >= f38280b

# 2. Real key in the environment (NEVER log the full value):
$env:ANTHROPIC_API_KEY = "<real key>"

# 3. Enable the native Windows CU executor explicitly (default-OFF), point the
#    manifests/verticals dirs at the repo, isolated ARGO_HOME + DB key, then
#    dispatch the Notepad task through the worker-action CU path
#    (WindowsCuConfig::for_notepad_app()), e.g. via `argo worker actions` against
#    a computer_use_windows-executor action, OR the gateway dispatch arm.
#    The loop writes per-monitor PNGs + hash-chained audit rows to the audit dir.

# Task to run: "Open Notepad and type 'SKYCOT Windows computer-use live run <UTC>',
# then leave it on screen."
# Expected ACT-floor behaviour to capture per the brief:
#   - app-launch / key (Win → "notepad" → Enter)  → ACT  → HOLD for approval
#   - type "<sentence>"                            → Observe → autonomous (no hold)
#   - focus key/click                              → ACT  → HOLD for approval
```

Then capture into this doc: the model's `CuAction` sequence, each action's
Observe/Act tier + approval decision, BEFORE/AFTER screenshots, the per-monitor
PNG sizes (real-pixel, #2919), the typed-sentence read-back, the hash-chained
audit rows, and the terminal `Completed`/`HumanHandoff` outcome.

---

## Honest account — what the Windows CU executor CAN vs CANNOT do (without a live run)

- **CAN (verified)**: compile + hermetic loop wiring (#2937, FakeRunner), real
  BitBlt→PNG screenshots (#2919), SendInput primitives (#2906/#2910), the
  `run()` orchestrator + `CuStepRunner` seam + DEC-051 ACT floor, and the
  resolve_executor dispatch arm calling real `run()`. The platform-journey
  (non-CU) gateway flow also serves live on Windows (`E2E-FULL-JOURNEY.md`),
  including a real LLM Genesis safety verdict via the local CLI.
- **CANNOT (without operator gates)**: a real model-in-loop desktop run — needs
  (1) a real `ANTHROPIC_API_KEY` and (2) a dedicated foreground desktop with an
  operator to approve ACT-floor holds. Neither is available to an autonomous
  shared-session worker. **This is a genuine operator-only gate, not a code gap.**

## Acceptance mapping

- Acceptance #1 (report exists with a genuine run OR a precisely-located honest
  gap): ✅ this doc is the precisely-located honest gap (no fabricated pass).
- Acceptance #4 (FEATURE_MATRIX Windows CU row): kept **🟡** with the honest
  "live run NOT performed — operator-gated (no key + no dedicated foreground
  session); #2937 wiring verified ready" note (NOT flipped to 🟢 — no live run
  completed).
- No code change (acceptance #3): correct — this brief RUNS the path; it found no
  bug to patch (the gate is operator-only, not a defect).

_Recorded by the Windows fleet (Latitude5440) on `f38280b`. The live run awaits
an operator with a key + a dedicated foreground desktop._
