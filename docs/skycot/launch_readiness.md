# SKYCOT launch readiness audit

A single-page summary of what's shipped, what's not, and the
confidence level on each. The view that should be in front of
you the day before first customer onboarding.

Last updated after PR #739 (worker doctor health-check).

## Ready for first customer

Per the user's mission ("Worker OS with generic execution +
approval-gated actions + artifacts + CLI flows + vertical
bindings"), every literal requirement has shipped:

| Capability                                  | Confidence | PR ref      | Locked in tests          |
|---------------------------------------------|------------|-------------|--------------------------|
| Vertical-declared action vocabulary         | High       | #729        | manifest classification lock test |
| Worker emits `action` JSON blocks           | High       | #729        | extract + orchestrator unit + integration |
| `actions.json` machine-readable artifact    | High       | #729        | render unit + integration |
| `execution_plan.md` markdown artifact       | High       | #729        | per-state grouping unit |
| Per-job artifact bundle (transcript / sections / approval_checklist / result.json) | High | #721-#729 | bundle layout integration |
| Approval CLI (`approve`, `approve-batch`)   | High       | #730        | unit + integration |
| State-machine-validated transitions         | High       | #730        | matrix lock unit + transition tests |
| `execute` / `complete` / `fail` / `status`  | High       | #730        | unit + integration |
| Workspace-level ledger files (approval_queue / completed_actions / blocked_actions) | High | #730 | render + integration |
| Auto-refresh after every transition         | High       | #735        | integration verifies files appear |
| Human-executable executor v1                | High       | #731        | unit (HumanExecutor::execute) + integration |
| Per-action artifact bundle                  | High       | #731        | unit + integration |
| Vertical-aware instruction templates        | High       | #732        | per-platform precedence unit + integration |
| Vertical lint for templates                 | Medium     | #736        | unknown-key unit; gtm-engineer manifest passes |
| `retry` failed actions                      | High       | #737        | integration (refuses non-failed; references source in evidence) |
| `argo worker doctor` health-check           | Medium     | #739        | unit (counts, dedup, recent-failed) |
| End-to-end smoke verification               | High       | #734        | 12-check shell script |

**Approval-gate enforced in code (the headline safety primitive):**

- `requires_approval = true` rows cannot transition `pending → executing` without `approve` first. Locked in `worker_actions_transitions::execute()` and tested in `execute_refuses_pending_approval_gated_action`.
- Insufficient research evidence (#727) downgrades approval-gated emissions to `blocked`. Locked in `emit_orchestrator_blocks_approval_action_when_evidence_insufficient`.
- Undeclared `action_type` lands `blocked` with capability-grant reason. Locked in `emit_orchestrator_preserves_undeclared_action_blocking_reason`.
- Manifest is canonical for `risk_level` + `requires_approval` — model cannot override. Locked in `build_emission_stamps_risk_and_requires_approval_from_manifest`.

**Customer-facing docs:**

- `docs/skycot/README.md` — what SKYCOT is + safety boundaries.
- `docs/skycot/getting_started.md` — install → first action in 10 minutes.
- `docs/skycot/demo_script.md` — 9-scene end-to-end walkthrough.
- `docs/skycot/operator_runbook.md` — full CLI reference + troubleshooting.
- `examples/marketplace-seller/` — 8 sample prompts, hand-crafted artifact bundle, 6-decision-point operator flow.
- `scripts/smoke-skycot.sh` — 12-check end-to-end install verification (mock provider; no API keys).

## Vertical bindings shipped

- `verticals/gtm-engineer/` — marketplace seller. 10 declared
  action types (verify_price, request_photos, post_listing,
  boost_listing, update_price, relist_item, mark_item_sold,
  reply_to_buyer, follow_up_buyer, create_listing_draft).
  Per-platform instruction templates for `ebay` and
  `facebook_marketplace` on `post_listing`.

## Not ready / out of scope for v1

| Capability                              | Why not                                                         | Plan                                              |
|-----------------------------------------|------------------------------------------------------------------|---------------------------------------------------|
| Browser-automation executor             | Browser MCP is stateful but no executor wired yet               | Post-v1: `BrowserExecutor` plugs into the trait. |
| Real marketplace SDK executors          | No vendor APIs wired                                            | Post-v1: per-platform `ApiExecutor`.              |
| Native voice TTS                        | Waiting on `piper-rs` upstream `ort` 2.x build                  | Decision-gated, not blocked.                      |
| OCI registry CI smoke                   | Needs test creds                                                | Decision-gated, not blocked.                      |
| Cloud browser backends (Browserbase)    | User-pays                                                       | Optional; per-customer decision.                  |
| Multi-vertical deployment isolation hardening | Memory rows are scoped per-vertical but no explicit cross-vertical RBAC | Add when first multi-tenant customer lands.   |
| Prometheus metrics for action ledger    | /metrics endpoint exists; action transitions don't publish yet | Wire into existing gateway/metrics.rs.            |
| `argo skycot init` convenience command  | Operators run 4 commands today (init / add / serve / list)      | Quality-of-life; not blocking.                    |
| TUI overlay for the action ledger       | TUI exists with 10 overlays; actions overlay TBD                | Quality-of-life; not blocking.                    |
| Vertical bindings beyond gtm-engineer   | AI Receptionist, Sales Follow-up, Support Triage manifests TBD | Per-customer; vertical authoring is data-only.    |

## Pre-launch checklist

Before the first paying customer:

- [ ] **Provider key sourcing** — operator must configure
      `argo setup` with their own Anthropic / OpenAI key.
      SKYCOT will not bring its own.
- [ ] **Vertical lint clean** — run `argo vertical lint <id>`
      against any custom verticals; fix warnings before they
      cause runtime confusion.
- [ ] **Smoke script passes** — `./scripts/smoke-skycot.sh`
      after install. 12 checks cover the operator surface.
- [ ] **Doctor confirms healthy** — `argo worker doctor`
      shows expected vertical loaded + zero unexpected
      failures or blocked rows in the ledger.
- [ ] **Read the demo script** — `docs/skycot/demo_script.md`
      walks the operator through a real run end-to-end before
      they touch their own data.
- [ ] **Approval gate test** — confirm in your own workspace
      that `argo worker actions execute` refuses an
      approval-gated action without `approve` first. The
      smoke script does this; verify locally too.

## CI status

Local verification across the worker stack:

- 70+ unit tests under `crates/argo-cli/src/handlers/worker_actions*` + `worker_doctor` + `worker_actions_executor` + `worker_actions_render` + `worker_actions_transitions`.
- 9+ integration tests under `crates/argo-cli/tests/work_action_*`.
- All authored files under the 500-LOC budget.
- `cargo fmt --all` + `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo test --workspace` exit 0 (modulo the pre-existing OCI port-binding flake which is documented in `MEMORY.md`).

Remote CI billing on the GitHub Actions side may be intermittently blocked — every PR's actual code is verified locally before merge.

## Confidence summary

> **The OS layer is launch-ready for the marketplace-seller use
> case.** The safety primitives that matter (approval gating,
> evidence gating, undeclared-action rejection, manifest-canonical
> classification) are enforced in code, not prompts, and locked
> in regression tests. The operator surface (CLI + artifact
> bundles + workspace ledger files + doctor + smoke script) is
> coherent and documented. The launch path is gated only on what
> third-party integrations the customer wants — none of those are
> in v1's contract.
