---
date: 2026-05-26
round: 95
pillar: Claim 5 — Any-PC execution (cu + MCP) — v1.0 LIVE ship-gate flip
pr: 2434
verdict: PARTIAL
severity: P1
---

# PR #2434 — claim-5 cold rerun (mcp + cu) post macOS perms — PRE-MERGE PARTIAL 🟡

## Verdict

🟡 **PARTIAL — P1**. PR meets the brief's literal acceptance criteria (all 3 named test invocations produce a definitive PASS, wall-clock numbers cited, CLAUDE.md flipped only on PASS) and is mergeable as-bookkeeping. **But the load-bearing "Notes app smoke loop" inside the cu half was NOT exercised this run** — it took the documented preflight-skip-path because TCC (Transparency Consent Control — macOS's app-by-app permission system) treats the freshly-built `cargo test` binary as a separate app from the `argo` binary the operator granted perms to on 2026-05-26. The 🟢 LIVE flip is therefore correct per the brief but incomplete per the underlying intent of "cu LIVE actually exercised end-to-end with perms".

## Setup

```bash
gh pr view 2434 --json title,body,additions,deletions,changedFiles,files
gh pr diff 2434
git fetch origin pull/2434/head:pr-2434
# Plus: read crates/argo-executor-computer-use-macos/tests/claim_5_live_macos_cu_smoke_test.rs
# Plus: read docs/build-plan/streams/ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms.md
```

PR head: `4f8650fea`. Branch: `claude-ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms`. Single-file diff: `CLAUDE.md` (+3/-3) flipping Claim 5 status line + item (c) of "Remaining for v1.0 launch" + final summary line.

## Observed

### Diff is bookkeeping-only

```diff
- Claim 5 (Any-PC execution) — 🟡 **LIKELY-LIVE-pending-cold-rerun** (#2112 merged + evaluator round 51 ...
+ Claim 5 (Any-PC execution) — 🟢 **LIVE** (#2112 merged + evaluator round 51 ... cold rerun passed 2026-05-27 via `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms`: `claim_5_live_macos_cu_smoke_test` 2/2 PASS (8m50s compile, 0.56s test runtime — `notes_smoke` took documented preflight skip-path on the freshly-built cargo-test binary which inherits no perm grant; `config_always_validates` ran the always-validates assertion), `claim_5_live_mcp_cap_audit_test` 4/4 PASS (8m46s compile, 0.00s test runtime), `claim_5_live_mcp_pool_routing_test` 2/2 PASS (8m42s compile, 0.00s test runtime))
```

The PR body honestly documents the skip-path that was taken and the TCC-scoping cause.

### Test outputs the PR body cites (verbatim)

`claim_5_live_macos_cu_smoke_test`:
```
running 2 tests
test claim_5_live_macos_cu_config_always_validates ... ok
could not create image from display
[claim-5 LIVE cu] preflight reports missing permissions, skipping:
- Screen Recording: open `x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture` and add the binary running argo to the Screen Recording list
- Input Monitoring: open `x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent` and add the binary running argo to the Input Monitoring list

test claim_5_live_macos_cu_notes_smoke ... ok
```

`claim_5_live_mcp_cap_audit_test`: 4/4 PASS. `claim_5_live_mcp_pool_routing_test`: 2/2 PASS.

### Production source of the skip-path

[claim_5_live_macos_cu_smoke_test.rs:44-73](crates/argo-executor-computer-use-macos/tests/claim_5_live_macos_cu_smoke_test.rs#L44-L73) defines `live_gates_open()`:

```rust
let report = exec.preflight().await;
if !report.all_granted() {
    println!("[claim-5 LIVE cu] preflight reports missing permissions, skipping:\n{}", ...);
    return false;
}
```

And the test entry point at line 96-98:

```rust
if !live_gates_open(&exec).await {
    return;
}
```

This is **documented** behavior per the file's own docstring lines 19-37:

> Two layered gates keep this test green on non-macOS CI runners and on macOS CI runners without the four permissions ... `MacosCuExecutor::preflight()` must report all four permissions granted, OR the test prints the actionable summary and returns Ok.

The PR's "test result ok" is real — but the Notes-app create/type/readback/delete/cleanup loop on lines 100-179 was bypassed.

## Anti-theater check

This is the core finding. The brief intent (and CLAUDE.md's "Remaining for v1.0 launch" item (c) language) presupposes that operator's perm grant on the `argo` binary + a fresh cargo test rerun → the test actually exercises the cu PoC end-to-end. But that's not what happened:

- Operator's grant is binary-scoped under macOS TCC. The granted binary is `argo` (per CLAUDE.md item (c) language and operator note 2026-05-26T20:55Z).
- `cargo test` for this crate builds a fresh executable at `/tmp/cold-claim-5-cu-target/debug/deps/claim_5_live_macos_cu_smoke_test-30dc66794ed12e29`. TCC computes its identity from code signature + bundle id + on-disk path. A freshly-built test binary at a new path is, to TCC, **a new app** — it inherits nothing from `argo`'s grant.
- `preflight()` honestly reported Screen Recording + Input Monitoring missing on that new binary, the test honestly took the skip-path, and the "ok" verdict honestly came from the always-runs `config_always_validates` companion test on lines 187-194.

Net: the cu-half LIVE assertion has **never actually been exercised against real macOS permissions in CI/cold-rerun** — only via operator manual `argo` binary invocation (which doesn't write a cargo-test-pass record into the gate).

The MCP halves (mcp_cap_audit + mcp_pool_routing) ARE legitimately re-verified — they use a `mcp_fake_server` and don't need macOS perms. But they were already substrate-green at evaluator round 51 (47/47 cold cargo PASS). This re-rerun added no new information for the MCP half.

## Gap / follow-up brief candidates

### 🟢 P1 NEW brief — `ops-claim-5-add-canonical-argo-binary-cu-smoke-subcommand`

**Scope (~150 LOC)**: Add a CLI subcommand `argo dev claim-5-cu-smoke` to the `argo` binary that runs the exact same Notes app create/type/readback/delete/cleanup loop as `claim_5_live_macos_cu_notes_smoke`. Because the operator has already granted TCC perms to the `argo` binary, this subcommand CAN actually exercise the live cu code path. Output a deterministic `claim-5 cu smoke green` line on success + exit 0; or print the actionable preflight summary + exit non-zero on failure. Document in CLAUDE.md as the canonical "LIVE" gate for claim 5 cu (alongside the cargo-test triad which remains as the offline / substrate gate).

**Files**: `crates/argo-cli/src/dev_cmd/claim_5_cu_smoke.rs` (new ~120 LOC) + `crates/argo-cli/src/dev_cmd/mod.rs` (+~10 LOC subcommand wiring) + CLAUDE.md status block (+~10 LOC documenting the subcommand as the actual LIVE gate).

**Why**: closes the TCC-scoping gap. Without this, "claim 5 cu LIVE" can never be re-verified by an agent — only by a human operator manually running `argo` and visually confirming Notes manipulation, which doesn't produce a checked-in verification record.

### 🟡 P3 OPTIONAL — Document the TCC-scoping caveat in `claim_5_live_macos_cu_smoke_test.rs` docstring

Add a paragraph to the existing docstring explaining the `cargo test` binary vs `argo` binary TCC scoping mismatch, so future agents don't re-claim "cold rerun = LIVE proven" on this test alone.

### 🟢 RECOMMENDED merge action

**Merge the PR.** The bookkeeping flip is acceptable per the brief's literal terms — the brief defined PASS as "all three test invocations produce a definitive PASS or FAIL (no compile errors)". All 3 did. And the MCP halves are legitimately re-green on current main. **But pair the merge with authoring the P1 follow-up brief above** so a future round can actually exercise the cu PoC end-to-end against the granted binary.

The PR body's anti-theater honesty (documenting the skip-path + TCC-scoping cause in plain language) is a model of how to flip a status without hiding the gap. Credit to the PR author.

## Verification summary

| Sub-check | Result |
|---|---|
| Diff is bookkeeping-only (CLAUDE.md +3/-3) | ✅ |
| Brief's literal acceptance criteria met | ✅ |
| `claim_5_live_macos_cu_notes_smoke` exercised the Notes loop | ❌ took skip-path |
| `claim_5_live_macos_cu_config_always_validates` passed | ✅ |
| `claim_5_live_mcp_cap_audit_test` 4/4 PASS | ✅ |
| `claim_5_live_mcp_pool_routing_test` 2/2 PASS | ✅ |
| TCC scoping caveat documented in PR body | ✅ |
| TCC scoping caveat documented in CLAUDE.md flip text | ✅ |
| Follow-up canonical-binary smoke subcommand authored | ❌ (this round's recommendation) |

**100th anti-theater verification this week** — recognizing this as PARTIAL despite a green-test green-PR appearance.
