# win-e2e-computer-use-run-loop-live

**Owner**: windows-fleet — **P1 — DEC-054 capstone: the FIRST real model-in-loop computer-use run on a live Windows desktop.**
**Depends on**: #2937 (Windows CU `run()` loop + risk-tiered ACT floor) MERGED; #2921 (resolve_executor wire) MERGED; #2906/#2910/#2919 (primitives + real PNG screenshot) MERGED.
**Authored against**: origin/main `059e974b9dc8566883d46a2366f4e37f4f0114c6` on `2026-06-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-e2e-computer-use-run-loop-live`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS (LATITUDE5440); cross-platform changes → conductor re-verifies unix before merge. Push branch `win-e2e-computer-use-run-loop-live`, flip the row to IN_REVIEW; **do NOT self-merge** — the macOS conductor gates/reviews/merges.

## Why this exists (and how it differs from #2937)
#2937 built the Windows CU `run()` loop and proved its PLUMBING with a HERMETIC test (a `FakeRunner` — no API key, no live desktop, no real model). That answered "does the loop wiring work?" It did NOT answer the real DEC-054 question: **does a teammate actually drive THIS Windows desktop — screenshot → real Anthropic model → real Windows input primitive → verify the effect — with the risk-tiered ACT floor (DEC-051) holding high-risk actions for approval?**

This brief is that run. It is the live counterpart to the still-BLOCKED `win-cu-executor-end-to-end-run` row, whose two operator-only gates were (1) a real `ANTHROPIC_API_KEY` and (2) firing synthetic input into a live Windows session. With the operator providing the key on the host and running the brief in a foreground desktop session, both gates clear. **This is a TEST brief: the deliverable is a DOCUMENTED real run with concrete, checkable evidence — or an HONEST account of exactly where the live path stops. NO THEATER: never a fabricated pass, never a mocked loop dressed up as live.**

## Premise — VERIFIED real on origin/main `059e974b9`
- `crates/argo-executor-computer-use-windows/src/executor.rs:186` — `pub async fn run(&self, req: WindowsCuRunRequest) -> WindowsCuResult<WindowsCuOutcome>` exists: preflight-gates (honest `HumanHandoff` with `report.actionable_summary()` on any failed axis), builds the per-run `AuditLog` + Anthropic client from `req.anthropic_api_key`, then drives `executor_loop::run_loop_with_client(...)`.
- `crates/argo-executor-computer-use-windows/src/executor_loop.rs:255` — `impl CuStepRunner for WindowsCuExecutor` (the shared turn/budget/risk-floor loop drives the Windows step primitives).
- `crates/argo-executor-computer-use-windows/src/cu_risk.rs` — the risk-tiered ACT floor (DEC-051): `classify_kind` puts `screenshot|cursor_position|wait|screen|read|ocr|enumerate|mouse_move|scroll|type` in `Observe` (autonomous) and **everything else — `click`, `key`, `double_click`, `navigate`, app-launch, and any UNKNOWN op — in `Act` (held for operator approval, fail-closed)**.
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_windows.rs:203` — the wired dispatch arm calls the real `exec.run(req)` (no longer the preflight+HumanHandoff stub).
- So the ONLY thing missing to make Windows a proven-live CU target is an actual run with a real key on a real desktop, with the evidence captured. That is this brief.

## Preconditions (operator-gated — the worker confirms each before running)
1. **`ANTHROPIC_API_KEY` is set in the host environment** (real provider traffic + real cost — operator must have provided it). If absent/empty, the run STOPS at "honest gap (no key)" and the report says so — do NOT skip to a fake pass.
2. **A live, foreground, interactive Windows desktop session** on LATITUDE5440 (not a headless/locked session) — the loop fires synthetic input into whatever is focused, so the operator must be present and the screen unlocked.
3. The release/debug `argo.exe` (or `cargo run -p argo-cli`) is built from `origin/main` at the freshness SHA (or newer); confirm with `argo --version` and record the SHA.
4. The CU executor is opt-in / default-OFF — confirm the same env/flag posture the macOS native executor uses (grep the resolve_executor arm + the `worker_actions_executor_cu_windows.rs` opt-in gate) and enable it explicitly for the run only.

## The e2e task to run (concrete, small, verifiable effect)
Drive ONE small, deterministic task end-to-end: **"Open Notepad and type the sentence `SKYCOT Windows computer-use live run <UTC timestamp>`, then leave it on screen."**

Why this task: Notepad is the canonical Windows CU smoke target (the crate already ships `WindowsCuConfig::for_notepad_app()` — see `worker_actions_executor_cu_windows_tests.rs:188`); the effect is unambiguously verifiable (the text is either in the Notepad window or it is not); and it exercises BOTH tiers of the ACT floor in one run:
- **app-launch / `key` (Win, type "notepad", Enter) → ACT** (must be held for approval at first occurrence),
- **`type` of the sentence → Observe / autonomous** (stages input, runs without an approval row),
- a follow-up **`key` or `click`** (e.g. to focus the edit area) → **ACT** again.

## Exact e2e steps (run on LATITUDE5440, foreground desktop, key present)
1. `git fetch origin && git rev-parse origin/main` — record the SHA the run is built against. `cargo build -p argo-cli` (or use the release `argo.exe`); `argo --version`.
2. Confirm preconditions 1–4 above; capture `echo %ANTHROPIC_API_KEY:~0,4%...` (FIRST 4 CHARS ONLY — never log the full key) to prove a key is present, and a `whoami` + "interactive session" note.
3. Pick a dedicated audit dir, e.g. `%TEMP%\skycot-cu-e2e\<UTC>` — the loop writes per-monitor PNG screenshots + audit rows there.
4. Take a BEFORE screenshot of the desktop (so the report shows the starting state).
5. Invoke the wired Windows CU dispatch path for the Notepad task (through `argo` worker-action CU dispatch / the `worker_actions_executor_cu_windows` arm with `WindowsCuConfig::for_notepad_app()` and the task prompt above). This is the REAL model-in-loop run: `run()` does preflight → builds the real Anthropic client from the key → `run_loop_with_client` drives screenshot → model → action → screenshot.
6. **At each ACT-tier action the loop should HALT for operator approval** (DEC-051 floor). Record the prompt the floor surfaces, and the operator's approve/deny decision, for EACH gated action. Approve the app-launch + focus actions so the task can complete; the `type` of the sentence should run WITHOUT an approval (it is `Observe`).
7. Let the loop run to a terminal state: `LoopExit::Success` → `WindowsCuOutcome::Completed`, or `LoopExit::Handoff` → `WindowsCuOutcome::HumanHandoff` (budget/handoff). Record which, and why.
8. Take an AFTER screenshot showing the Notepad window with the typed sentence (the verifiable effect). If `read_clipboard`/notes read-back is available (#2910 style), also capture a programmatic read-back of the typed text.
9. Collect the audit dir: list the per-monitor PNGs (sizes + that they decode as real PNGs, leaning on #2919), and the hash-chained audit rows the run wrote (action kind, summary, ACT/Observe tier, approval decision, result).

## CHECKABLE evidence the report MUST contain (NO THEATER)
The report at `docs/skycot/windows/E2E-CU-RUN-LIVE.md` must contain, all as concrete artifacts (not prose claims):
1. **Provenance**: the origin/main SHA the run was built against, `argo --version`, host = LATITUDE5440, UTC timestamp, and the FIRST-4-CHARS-ONLY proof a real key was present (key value NEVER logged).
2. **The model's chosen actions, in order** — the actual `CuAction` sequence the Anthropic model emitted (screenshot → app-launch → type → … → stop), with each action's `action_kind` label.
3. **The ACT-floor gating decisions** — for EACH action, its `Observe`/`Act` classification and, for every `Act` action, the approval prompt that was surfaced + the operator's approve/deny. Proof the floor actually held (e.g. the `type` sentence ran with NO approval row, the `key`/launch/`click` actions DID require one). An honest note if any action classified UNKNOWN→ACT (fail-closed).
4. **Screenshots**: BEFORE (desktop) and AFTER (Notepad with the typed sentence), embedded or path-referenced from the audit dir, plus the per-monitor PNG sizes + that they decode to real images (real-pixel, not the old 16-byte stub).
5. **The verifiable effect**: the typed sentence is present in Notepad (screenshot) AND, if available, a programmatic read-back string match.
6. **The audit rows** the run wrote (hash-chained), showing action kind / tier / approval / result per step — proof the run is auditable, not narrated.
7. **The terminal outcome**: `Completed` vs `HumanHandoff`, with the wallclock + step count + budget consumed.
8. **HONEST account of what the Windows CU executor CAN vs CANNOT do live** — every place the live path fell short of macOS parity (an action kind `events::post` can't drive yet → honest `CuError`/handoff, NOT a fake Ok; any preflight axis that failed; clipboard read-back unavailable; etc.). If the run can't complete the Notepad task at all, the report says exactly where it stopped and why — that is a VALID, valuable result. A truthful "we got to step N then hit X" beats any fabricated green.

## Stop conditions / honest-gap paths (each is a valid result, none is a failure to fake around)
- **No key** → STOP at precondition 1; report "honest gap — no ANTHROPIC_API_KEY on host; live run not run". (This keeps the brief HONEST if the operator hasn't provisioned the key.)
- **Preflight fails** (an axis like a11y/screen-capture not granted) → `run()` returns `HumanHandoff` with `actionable_summary()`; record the summary verbatim — that IS the result.
- **An action kind can't be driven** by `events::post` → honest `CuError`/handoff for that action; record it; do NOT substitute a mocked success.
- **Budget exhaustion** → `HumanHandoff`; record the budget trace.

## Acceptance criteria
1. `docs/skycot/windows/E2E-CU-RUN-LIVE.md` exists and contains items 1–8 of "CHECKABLE evidence" above — either a genuine completed live run OR a precisely-located honest gap (no fabricated pass).
2. If the run completed: the AFTER screenshot + read-back show the sentence in Notepad, the action trace + audit rows are present, and the ACT-floor gating decisions are recorded per action (proof DEC-051 held live).
3. No code change is required by this brief (it RUNS the #2937 path). If running surfaces a real bug (e.g. an action `events::post` mis-drives, or the floor mis-classifies), the report records it precisely and recommends a NEW follow-up brief — it does NOT silently patch and claim a pass.
4. `docs/build-plan/FEATURE_MATRIX.md` row for `Computer-use — Linux / Windows` updated IN-BAND in the same commit: flip the Windows half toward "🟢 Live model-in-loop CU run verified on Windows (Notepad task, ACT-floor gated) — see E2E-CU-RUN-LIVE.md" IF the run completed, or keep it 🟡 with an honest "live run attempted; stopped at <gap>" note if it did not. Cite the commit SHA + the report path. Do NOT commit the generated feature-matrix HTML.
5. Branch `win-e2e-computer-use-run-loop-live` pushed; ledger row flipped to IN_REVIEW with the branch name. **Do NOT self-merge** — the macOS conductor gates.

## Parallel-surface declaration
This brief adds NO new pub surface — it RUNS the existing `WindowsCuExecutor::run()` / `run_loop_with_client` / `cu_risk::classify` path from #2937 and produces a report doc. `rg 'pub (struct|enum|type)'` is N/A (no new types). The only new file is the report `docs/skycot/windows/E2E-CU-RUN-LIVE.md` (and any captured screenshots/audit artifacts referenced from it).

## NO THEATER (load-bearing)
This is a LIVE run on real hardware with a real model and real cost. The whole value is the truth signal: a documented real run, or an honestly-located stopping point. A mocked loop, a hand-typed sentence passed off as model-driven, a screenshot staged by hand, or a "Completed" with no audit rows — all are theater and all are rejected at review. The macOS conductor adversarially reviews this PR specifically for fabrication.