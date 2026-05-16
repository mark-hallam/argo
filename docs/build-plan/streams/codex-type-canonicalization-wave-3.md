# codex-type-canonicalization-wave-3 — Canonicalize 33 duplicate public types + enforce CI lint (F-08 P1)

**Owner**: codex. **Depends on**: `codex-ci-workflows-reactivate` MERGED (so the enforced lint actually fires).
**Authored against**: origin/main `58e853b6` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-type-canonicalization-wave-3`.
**Source**: F-08 P1 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Mission alignment

- **`CLAUDE.md:111`** locks type canonicalization as an operator v1 decision.
- **`DRIFT_PREVENTION.md:91`** documents the parallel-type lint.
- **`CLAUDE.md:173`** says duplicate overlapping public types are a bug.
- **Foundational invariant** — supports all 6 mission claims via consistent contracts across audit/policy/provider/workflow/RBAC boundaries.

## Scope

P1 cleanup. `python3 scripts/lint-parallel-pub-types.py` reports **33 duplicate public type names**, regressed vs Phase A baseline. High-risk examples (codex audit):

| Type | Locations |
|---|---|
| `AuditEvent` | `argo-audit/src/sink.rs:71`, `argo-siem/src/sink.rs:13` |
| `AuditRow` | `argo-cli/.../routes_worker_overview_kpis.rs:23`, `argo-store/src/models.rs:153` |
| `AuditSink` | `argo-audit/src/sink.rs:59`, `argo-mcp-image-gen/src/server.rs:35` |
| `BackendKind` | `argo-cli/.../worker_actions_executor_code.rs:112`, `argo-mcp-image-gen/src/config.rs:8` |
| `BudgetSnapshot` | `argo-executor-computer-use/src/budget.rs:150`, `argo-tui/src/spend_monitor.rs:26` |
| `ComputerUseExecutor` | `argo-cli/.../worker_actions_executor_stubs.rs:78`, `argo-executor-computer-use/src/executor.rs:41` |
| `MemoryLayer` | `argo-genesis/src/memory_schema.rs:30`, `argo-memory/src/layers.rs:24` |
| `PendingApproval` | `argo-cli/.../audit_report_aggregate.rs:33`, `argo-tui/src/approval_tray.rs:13`, `argo-voice-tui/src/runner.rs:24` |
| `Principal` | `argo-cli/.../routes_auth_principal.rs:14`, `argo-types/src/capability.rs:255` |
| `ProviderRequest` / `ProviderResponse` | `argo-executor-document/src/extract/provider.rs` vs `argo-provider/src/transport.rs` |
| `StopReason` | 3 crates |
| `ToolSchema` | `argo-provider/src/transport.rs:52`, `argo-tool/src/tool.rs:156` |
| `TrustGrade` | `argo-skill/src/trust.rs:33`, `argo-store/src/learned_plan_template.rs:72` |
| `Workflow` | `argo-vertical/src/workflow.rs:60`, `argo-workflow/src/dag.rs:100` |

Scope (per audit's recommendation, security/audit/provider/workflow first):
- Triage each duplicate cluster: identify canonical home (usually `argo-types` or the owning domain crate) and downstream consumers.
- For each: pick canonical, re-export from canonical, replace downstream `pub struct X` declarations with `pub use canonical::X`, fix any field/method mismatches.
- For local-scope-only shadows (e.g., `pub(super) struct AuditRow` in a gateway route — same name but truly module-scoped) — RENAME the shadow rather than canonicalize. The lint should still pass after rename.
- Update `scripts/lint-parallel-pub-types.py` to enforce (not warn) for any cluster NOT in an explicit allowlist; the allowlist is reviewed + checked in.
- Wire the enforced lint into the F-01-reactivated CI workflow.

Priority canonicalization order (per audit + mission impact):

**Wave A (security/audit, mission-load-bearing):**
- `AuditEvent`, `AuditRow`, `AuditSink`, `Principal`, `PendingApproval`, `TrustGrade`

**Wave B (provider/transport):**
- `ProviderRequest`, `ProviderResponse`, `StopReason`, `ToolSchema`

**Wave C (workflow/computer-use/memory/etc):**
- `Workflow`, `BackendKind`, `BudgetSnapshot`, `ComputerUseExecutor`, `MemoryLayer`

**Wave D (anything else from the 33-list):**
- Per lint output. Tail wave.

LOC: ~800 (this is the largest brief in the audit's roadmap; matches the scope).

## Files you may touch

- `crates/argo-types/src/*.rs` — receive canonical exports for cross-cutting types. ≤200 modified lines.
- For each duplicate cluster: the 2-3 source files (one canonical, others get `pub use`). ≤500 modified lines combined.
- `scripts/lint-parallel-pub-types.py` — flip warn → enforce; add allowlist YAML. ≤80 modified lines.
- `docs/skycot/type-canonicalization-allowlist.yaml` (NEW) — reviewed allowlist for any intentional duplicates (expect to be short). ≤50 LOC.
- Test additions: type-equality assertions for each canonical type's downstream consumers. ≤150 LOC combined.

## Files you must NOT touch

- Field shapes that differ across the duplicates — if `AuditRow` in argo-store has different fields than the one in a gateway route, that's a SEMANTIC mismatch worth flagging in PR body. Don't paper over with mismatched structs.
- Public APIs of crates not in the duplicate list — out of scope.
- `argo-mcp-image-gen` if its `AuditSink` and `BackendKind` are genuinely different concepts (operator decides) — rename them with crate prefix (e.g., `ImageGenBackendKind`) rather than force-canonicalize.

## Acceptance criteria

- [ ] `python3 scripts/lint-parallel-pub-types.py` returns ZERO duplicates (excluding allowlist) — or `1`-`5` justified entries in the allowlist with comments.
- [ ] Security/audit cluster (Wave A) canonicalized — verify by grep that there's exactly ONE `pub struct AuditRow` (or `pub use argo_X::AuditRow as ...`) per crate.
- [ ] Provider/transport cluster (Wave B) canonicalized — same proof.
- [ ] Workflow/CU cluster (Wave C) canonicalized.
- [ ] Lint script enforces (not warns) — `python3 scripts/lint-parallel-pub-types.py --enforce` exits non-zero on new duplicates outside allowlist.
- [ ] CI workflow (post-F-01) invokes the enforced lint.
- [ ] `cargo test -p argo-types -p argo-audit -p argo-provider -p argo-skill -p argo-cli` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-type-canonicalization-wave-3`
- PR title: `refactor(types): canonicalize 33 duplicate pub types + enforce lint (codex-type-canonicalization-wave-3) — F-08 P1`

## Notes for the implementer

- This is a large brief but each cluster is mechanical. Tackle by Wave A → B → C → D.
- For each duplicate, the questions are: (a) are these actually the same concept? (b) which crate owns the canonical? (c) what do downstream consumers need to do?
- If two types share a name but are semantically different (e.g., a UI-side `PendingApproval` view-model vs an audit-side `PendingApproval` record), RENAME the smaller-blast-radius one with a prefix (e.g., `AuditPendingApproval`, `TuiPendingApproval`). Don't force canonical when concepts differ.
- The lint allowlist is for justified-different-concept cases. Keep it short; document each entry.
- After this brief, F-08 closes and CI structurally prevents recurrence.
