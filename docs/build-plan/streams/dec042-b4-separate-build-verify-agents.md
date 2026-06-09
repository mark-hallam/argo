# dec042-b4-separate-build-verify-agents

**Owner**: claude.
**Authored against**: origin/main `7c81986c2` on `2026-06-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh dec042-b4-separate-build-verify-agents`.
**Depends on**: `dec042-b3-automated-rule-capture` (#2756) MERGED — DEC-042 §5
locks the Stage-1 order B-1 → B-2 → B-3 → B-4. **Row stays BLOCKED until
#2756 merges.**

> DEC-042 Stage 1, upgrade **B-4** (the last build-loop hardening step; B-1
> #2753 MERGED, B-2 #2755 MERGED, B-3 #2756 in review). See
> [`docs/skycot/decisions/DEC-042-continuous-self-improvement-loop.md`](../../skycot/decisions/DEC-042-continuous-self-improvement-loop.md)
> §3 B-4 + §5. DEC-042 §3 B-4 verbatim: *"the agent that wrote the code is
> biased when grading it ... make a separate verify-agent a required stage of
> every loop ... Verifiable by: every merged change carries a verify-agent
> verdict from a different context than the author."*
>
> Merge policy: like B-1/B-2 this is pure additive default-OFF ops tooling
> (a new flag nobody sets yet) — normal conductor review, no operator-hold
> needed beyond the standard gate.

## Why (VERIFIED 2026-06-10, read-only)
Today the only worker-side verification is B-2's stop-gate, and it runs **in
the same context that built the change**:

- `scripts/claude-worker-loop.sh` sources `stop-gate-enforce.sh` (lines
  155-157) and calls `stop_gate_enforce` after the ship-check (lines
  839-843, behind default-OFF `ARGO_STOP_GATE`).
- `stop_gate_enforce` re-runs `scripts/stop-gate-verify.sh` — four
  DETERMINISTIC gates (LOC budget / ledger-in-band / cargo test / workspace
  clippy). Deterministic gates are author-independent, which is good — but
  nothing reviews the change **as an agent with fresh context** (does the
  diff actually satisfy the brief's acceptance criteria? is the test
  asserting the right thing or theater?). The builder session
  (`claude -p`, spawned at worker-loop line 393) grades its own
  acceptance-criteria comprehension.
- `rg -n 'ARGO_SEPARATE_VERIFIER|verify-agent|VERIFY_AGENT' scripts/` — zero
  hits; B-4 is unauthored and there is no parallel surface.
- The conductor's pre-merge gate is a second *context*, but it is also
  deterministic-only and its verdict is not bound per-PR as a required,
  visible stage — the DEC-042 §3 B-4 "verifiable by" artifact (a
  verify-agent verdict on every change, from a non-author context) does not
  exist anywhere.

## Grep-before-write
- `rg -n 'stop_gate_enforce|ARGO_STOP_GATE' scripts/claude-worker-loop.sh scripts/stop-gate-enforce.sh scripts/stop-gate-verify.sh` — the B-2 pattern to MIRROR (default-OFF flag, sourced helper, NEEDS-FIX row flip, PR comment, env test-seams). Do not author a parallel enforcement mechanism; B-4 composes WITH the stop-gate.
- `rg -n 'STOP_GATE_VERIFY_CMD|STOP_GATE_SKIP_PR_COMMENT' scripts/` — the established test-seam naming convention to mirror (`VERIFY_AGENT_CMD`, `VERIFY_AGENT_SKIP_PR_COMMENT`).
- `ls scripts/test-stop-gate.sh` — the B-2 meta-verifier whose structure (throwaway ledger fixture, injected gate outcomes, run-twice determinism) the new meta-test mirrors.

## Fix (a second agent, fresh context, default-OFF)
1. **`scripts/stop-gate-verify-agent.sh`** (NEW, < 200 LOC — joins the
   `stop-gate-*` family). Contract mirrors `stop-gate-verify.sh`:
   - No-op `VERIFY_AGENT_PASS` / exit 0 unless `ARGO_SEPARATE_VERIFIER=1`
     (byte-identical loop behavior with the flag unset — the
     no-behavior-change proof, same discipline as B-2).
   - Args: `--brief <id> --branch <branch> [--pr <n>] [--repo <path>]`.
   - When armed: spawn a **fresh `claude -p` verify-only session** — a
     different context from the builder by construction (new process, new
     context window, read-only prompt). The prompt: read
     `docs/build-plan/streams/<brief>.md` acceptance criteria + the branch
     diff vs `origin/main`, judge whether the diff satisfies the criteria
     and whether the tests added actually pin the claimed behavior; it must
     answer with a machine-parseable LAST line: `VERIFY_AGENT_PASS` or
     `VERIFY_AGENT_FAIL: <one-line reason>` (same output contract style as
     stop-gate-verify). The session gets NO write tools (read-only review;
     `claude -p` with a review prompt, no `--dangerously-skip-permissions`).
   - Env seams (so the meta-test injects outcomes with NO real model call,
     no network): `VERIFY_AGENT_CMD` replaces the claude invocation;
     `VERIFY_AGENT_SKIP_PR_COMMENT=1` suppresses `gh pr comment`.
   - Bounded: wall-clock timeout (default ~10 min, `VERIFY_AGENT_TIMEOUT_S`)
     — on timeout emit `VERIFY_AGENT_FAIL: verify session timed out`, never
     hang the worker loop.
2. **Verdict is the §3 B-4 artifact.** On either verdict, post a PR comment
   (`🔎 verify-agent (DEC-042 B-4): PASS/FAIL — <reason>`) so every change
   carries a visible verify-agent verdict from a non-author context. On
   FAIL: flip the ledger row IN_REVIEW→NEEDS-FIX with the reason (reuse the
   row-flip approach in `stop-gate-enforce.sh` — extract its python row-flip
   into a small shared helper or mirror it; do NOT fork divergent
   ledger-mutation logic silently — if mirroring, add a comment
   cross-linking the two sites).
3. **Wire into `claude-worker-loop.sh`** directly AFTER the existing
   `stop_gate_enforce` block (≤ ~15 added lines — the loop is already 902
   LOC; all logic lives in the new script): only runs when
   `ARGO_SEPARATE_VERIFIER=1` AND the stop-gate did not already hold the row
   (deterministic gates first — cheap and unambiguous; the agent pass is the
   judgment layer on top).
4. Ordering note for the brief's implementer: B-4 intentionally does NOT
   make the verify-agent a merge gate by itself — the conductor still
   merges (DEC-034). It makes the independent verdict EXIST and be visible
   on every PR; flipping it to merge-blocking is a later operator call.

## Acceptance (truth-bounded — checkable verifier)
1. **`scripts/test-verify-agent.sh`** (NEW, < 150 LOC) mirroring
   `test-stop-gate.sh`: throwaway ledger fixture with one IN_REVIEW row;
   cases:
   - (A) `ARGO_SEPARATE_VERIFIER=1` + injected `VERIFY_AGENT_CMD` FAIL →
     row flipped to NEEDS-FIX, reason recorded, nonzero return.
   - (B) flag ON + injected PASS → row stays IN_REVIEW, return 0.
   - (C) flag UNSET → the verify command is NEVER invoked (assert via a
     side-effect file the injected cmd would create), return 0 — the
     no-behavior-change proof.
   - (D) output-contract parse: a FAIL last-line yields the reason verbatim
     in the row note.
   - Suite runs 2× with identical results; deterministic; no network; no
     real `claude` invocation.
2. `bash -n` + `shellcheck` clean on the new script + the loop diff (no NEW
   findings).
3. With `ARGO_SEPARATE_VERIFIER` unset, `scripts/test-stop-gate.sh` still
   passes unchanged (B-2 regression pin).
4. All files < 500 LOC.

## On ship (same commit)
- `FEATURE_MATRIX.md` row: "B-4 separate build/verify agents (default-OFF
  `ARGO_SEPARATE_VERIFIER`; verify-agent verdict on every PR)". Do NOT
  commit the generated feature-matrix.html (DEC-043).
- Note it completes DEC-042 Stage 1 (B-1 ✅ B-2 ✅ B-3 ✅ → **B-4 done**);
  per DEC-042 §10 the conductor should tag
  `checkpoint/dec042-stage1-verified-<date>` once all four are verified.

## Effort
M (one new bash script + ~15-line loop wire + one meta-test; no Rust, no
web; default-OFF).
