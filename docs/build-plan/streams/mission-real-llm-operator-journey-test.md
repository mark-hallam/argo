# mission-real-llm-operator-journey-test — `smoke-skycot.sh` companion that exercises the full path against a REAL provider

**Owner**: claude. **Depends on**: nothing — extends the existing mock-mode `scripts/smoke-skycot.sh`. Independent of `mission-extended-smoke-script` (which is BrowserExecutor + Slack-approval focused).
**Authored against**: origin/main `70e4b020` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-real-llm-operator-journey-test`.

## Why

Today the only end-to-end test is `scripts/smoke-skycot.sh`, which runs in **mock-mode** (`--provider mock`) with no API calls. That validates the wiring but tells you nothing about whether the worker actually produces coherent output against a real LLM. The mission claims "AI workers that humans can bring into their team" — claim #1 is unfalsifiable without a real-LLM run.

This brief ships a second smoke script that runs the same path against a **real provider** (Anthropic or OpenAI), asserts the worker produced a non-empty action with a coherent (non-empty, > N chars, no obvious mock-text) description, and reports token cost. The script **skips cleanly with exit 0** when no provider key is set, so it can sit in CI without breaking key-less environments — but emits a clear "RAN" vs "SKIPPED" header so the operator can see at a glance whether real-LLM coverage executed.

This is the script the operator runs on a fresh machine after setup to confirm "yes, the agent actually thinks". It is the first deliverable the operator can hand a new user to prove the platform is real.

## Scope

NEW `scripts/smoke-skycot-real-llm.sh`. Structure mirrors `scripts/smoke-skycot.sh` exactly (same heading-style stage logging, same `ok` / `fail` / `heading` shell helpers, same tempdir + trap pattern). Differences:

- **Provider selection**: reads `ANTHROPIC_API_KEY` first, then `OPENAI_API_KEY`. Exits 0 with `=== SKIPPED: no provider key set ===` if neither is set. (Do not fail — this script is opt-in.)
- **Job content**: queues a single concrete `gtm-engineer` job — `"research the current price of Bauer Vapor X5 inline skates and draft a marketplace listing description"` — chosen because gtm-engineer has the most mature action vocabulary and the task has a real, verifiable outcome.
- **Steps**: `argo work serve --vertical gtm-engineer --once --max-steps 5` (NOT max-steps 1 — real LLM needs room for plan + research + draft).
- **Cost reporting**: capture the JSON serve output, extract `tokens_input` / `tokens_output` / `cost_usd` if present, print a final `=== COST: $X.XX (input=N output=M) ===` line. If those fields are not yet emitted by `work serve --json`, log a clear `(cost reporting not yet available; see #<pr>)` line and **do not fail** — this is forward-compatible.
- **Coherence assertion**: read the action description text from the worker ledger; assert it is non-empty, ≥ 40 characters, and does NOT contain the literal substring `"mock"` or `"placeholder"` (case-insensitive). The mock provider in `smoke-skycot.sh` produces deterministic stub text — this assertion proves we are NOT seeing it.
- **Artifact assertion**: the `.skycot/outputs/<job_id>/` bundle exists and contains at least one file > 100 bytes.
- **Total runtime budget**: 90 seconds. Fail with `BUDGET EXCEEDED: real-LLM run took N seconds (cap 90)` if exceeded — this catches stuck inference loops in CI.

The script must be runnable as `./scripts/smoke-skycot-real-llm.sh [argo-binary]` exactly like `smoke-skycot.sh`.

## Files you may touch

- `scripts/smoke-skycot-real-llm.sh` (NEW, ≤ 250 LOC). Match the bash style of `smoke-skycot.sh` exactly.
- `docs/skycot/getting_started.md` — narrowly scoped: add a section "## 8. (optional) Verify with a real LLM" linking to the new script. Do NOT renumber existing sections; append.
- `README.md` (repo root) — narrowly scoped: one bullet under the smoke-test mention pointing at the new script.

## Files you must NOT touch

- `scripts/smoke-skycot.sh` — leave the mock-mode script byte-identical. Any shared helper extraction is OUT of scope; we accept the duplication for v1.
- `crates/**` — this is a script-only brief; no Rust changes. If the worker discovers a missing flag (e.g. `--json` output doesn't carry token counts yet), document the gap with a follow-up brief stub in the PR description; do NOT add the flag in this PR.
- `verticals/gtm-engineer/**` — use the existing manifest as-is. Do NOT modify prompts to "tune" them for the test.
- Any CI workflow file. Wiring this into CI is a separate brief — depends on provider-key availability decisions that aren't this brief's call.

## Acceptance criteria

- [ ] `./scripts/smoke-skycot-real-llm.sh` with NO provider key set: exits 0, prints `=== SKIPPED: no provider key set ===` with installation hint
- [ ] `./scripts/smoke-skycot-real-llm.sh` with `ANTHROPIC_API_KEY=<valid>` set: exits 0, runs the full path, asserts non-mock coherent output, prints cost line
- [ ] `./scripts/smoke-skycot-real-llm.sh` with `OPENAI_API_KEY=<valid>` (Anthropic key absent): exits 0, runs against OpenAI provider, same assertions
- [ ] Output ledger heading clearly shows one of three states: `RAN (anthropic)`, `RAN (openai)`, or `SKIPPED`
- [ ] 90-second wall-clock budget enforced; exceeding it fails with a clear message
- [ ] Coherence assertion: action description ≥ 40 chars AND no `"mock"`/`"placeholder"` substring
- [ ] Output bundle exists and at least one artifact file > 100 bytes
- [ ] Script passes `shellcheck` (the same gate `smoke-skycot.sh` passes, if any)
- [ ] `docs/skycot/getting_started.md` has the new section linking the script
- [ ] README has the new bullet
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-real-llm-operator-journey-test`
- PR title: `tools(smoke): real-LLM operator journey test script (mission-real-llm-operator-journey-test)`

## Notes for the implementer

- Read `scripts/smoke-skycot.sh` end-to-end first. Match the `heading` / `ok` / `fail` helpers; reuse the tempdir + trap pattern; preserve the cwd-reset behavior at exit.
- Do NOT use `set -e`. `smoke-skycot.sh` uses `set -uo pipefail` deliberately so failure messages can attribute the offending command. Match it.
- Do NOT hardcode a model id (`claude-sonnet-4-6`, `claude-opus-4-7`, etc.). Use `argo`'s default for whichever provider key is present — let the binary pick. Hardcoding is a freshness hazard.
- The cost line is **forward-compatible**: if `work serve --json` does not yet emit `cost_usd`, print `(cost reporting not yet available — see follow-up)` and continue. Adding the cost output to `work serve` is a separate brief.
- The "mock-text" string list — `"mock"`, `"placeholder"` — should be a small bash array near the top of the script so future operators can add to it.
- Do NOT pull in `jq` as a hard dependency. Use `grep -oE` patterns the way `smoke-skycot.sh` does (`grep -oE '"job_id":"[^"]+"'`). The script must run on a vanilla mac/Linux/Windows-WSL box.
- Test on YOUR own machine with a real key BEFORE opening the PR. Paste the actual `=== COST: $X.XX ===` line into the PR description — this proves it works against a real provider, not just compiles.
- If the worker decides to call computer-use mid-run (because the gtm-engineer task includes browser research), that's expected. Budget for it in the 90s cap.
- The skip path matters: this script is meant to be safe-to-run in CI later. A flaky skip = a failing pipeline. Test the skip path with `unset ANTHROPIC_API_KEY OPENAI_API_KEY && ./scripts/smoke-skycot-real-llm.sh`.
