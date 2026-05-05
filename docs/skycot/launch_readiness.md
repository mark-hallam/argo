# SKYCOT launch readiness audit

A single-page summary of what's shipped, what's not, and the
confidence level on each. The view that should be in front of
you the day before first customer onboarding.

Last updated after PRs #729–#759 (Worker v3 generic execution
through to provider-keys docs, Prometheus metrics for the
action ledger, retry policy, failure-modes reference, and the
eval suite expansion across the cohort).

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
| `argo worker doctor` health-check           | High       | #739, #740  | unit (counts, dedup, recent-failed) + smoke check #13 |
| `argo work jobs list` — read-side queue     | High       | #742        | unit (pending/completed merge + filter) |
| Cross-vertical isolation                    | High       | #741        | end-to-end integration: actions emitted under A invisible to B |
| Second vertical binding (AI Receptionist)   | High       | #743        | manifest classification lock + lint-clean integration |
| Third vertical binding (Sales Follow-up)    | High       | #746        | manifest classification lock + lint-clean integration |
| End-to-end smoke verification               | High       | #734, #740, #749 | 17-check shell script: lint + eval + setup --check all 3 v3 verticals |
| Retry policy w/ max_attempts + force        | High       | #755        | unit (count_retry_chain) + integration (force override) |
| Canonical failure-modes reference           | High       | #756        | docs/skycot/failure_modes.md — 11 modes |
| Prometheus + JSON metrics for action ledger | High       | #757        | unit (gather/dedupe/render) + gateway scrape |
| Provider-keys + production secrets flow     | High       | #758        | docs/skycot/provider_keys.md — env / systemd / k8s / Vault |
| Eval suite expansion across v3 cohort       | High       | #759        | structural CI lock for all 3 verticals (≥7 fixtures each) |

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
- `docs/skycot/failure_modes.md` — 11 failure modes with symptom / cause / recovery / prevention shape (#756).
- `docs/skycot/provider_keys.md` — local-dev secrets flow + production patterns (env / systemd / k8s Secret / Vault) + key rotation + production launch checklist (#758).
- `docs/skycot/CHANGELOG.md` — customer-facing release notes per SKYCOT version.
- `examples/marketplace-seller/` — 8 sample prompts, hand-crafted artifact bundle, 6-decision-point operator flow.
- `examples/ai-receptionist/` — 6 inbound triage prompts + sample bundle.
- `examples/sales-follow-up/` — 5 lead scenarios + sample research / draft / approval checklist.
- `scripts/smoke-skycot.sh` — 17-check end-to-end install verification (mock provider; no API keys).

## Vertical bindings shipped

- **`verticals/gtm-engineer/` — marketplace seller.** 10
  declared action types (verify_price, request_photos,
  post_listing, boost_listing, update_price, relist_item,
  mark_item_sold, reply_to_buyer, follow_up_buyer,
  create_listing_draft). Per-platform instruction templates
  for `ebay` and `facebook_marketplace` on `post_listing`.

- **`verticals/ai-receptionist/` — front-desk worker (#743).**
  6 declared action types. Autonomous-safe: `lookup_contact`,
  `draft_meeting_request`, `take_message`. Approval-gated:
  `send_calendar_invite` (with a Google-Calendar-specific
  template override), `forward_call`, `send_followup_email`
  (HIGH RISK).

- **`verticals/sales-follow-up/` — warm-lead follow-up
  worker (#746).** 6 declared action types. Autonomous-safe:
  `research_lead`, `find_contact`, `draft_followup`.
  Approval-gated: `schedule_followup_cadence`, `log_to_crm`
  (per-platform templates ready for hubspot / salesforce /
  close), `send_followup_email` (HIGH RISK).

All three shipped verticals share zero code beyond the OS-level
Worker v3 contract — proving vertical authoring is genuinely
TOML-and-prompt-only.

## Not ready / out of scope for v1

| Capability                              | Why not                                                         | Plan                                              |
|-----------------------------------------|------------------------------------------------------------------|---------------------------------------------------|
| Browser-automation executor             | Browser MCP is stateful but no executor wired yet               | Post-v1: `BrowserExecutor` plugs into the trait. |
| Real marketplace SDK executors          | No vendor APIs wired                                            | Post-v1: per-platform `ApiExecutor`.              |
| Native voice TTS                        | Waiting on `piper-rs` upstream `ort` 2.x build                  | Decision-gated, not blocked.                      |
| OCI registry CI smoke                   | Needs test creds                                                | Decision-gated, not blocked.                      |
| Cloud browser backends (Browserbase)    | User-pays                                                       | Optional; per-customer decision.                  |
| Multi-vertical deployment isolation hardening | Memory rows are scoped per-vertical but no explicit cross-vertical RBAC | Add when first multi-tenant customer lands.   |
| Per-tenant spend caps (auto-throttle at $X/hour) | `/metrics` exposes `argo_spend_usd_micros_total`; no in-process budget primitive yet | Operator monitors via `/metrics` + alerts; budget primitive is a v1.x add. |
| `argo skycot init` convenience command  | Operators run 4 commands today (init / add / serve / list)      | Quality-of-life; not blocking.                    |
| TUI overlay for the action ledger       | TUI exists with 10 overlays; actions overlay TBD                | Quality-of-life; not blocking.                    |
| Live-LLM eval scoring per vertical      | Structural eval suites land in #759; populating actuals against a real provider is a manual / per-customer task | Operator runs `argo vertical eval --score` after `--actuals` is wired. |

## Pre-launch checklist

Before the first paying customer:

- [ ] **Provider keys wired per `provider_keys.md`** — operator
      configured `argo setup` (local) or env vars / k8s Secret
      / systemd unit / Vault wrapper (production). Both
      `ANTHROPIC_API_KEY` AND the fallback provider key
      (`OPENAI_API_KEY` by default) are loadable from the
      worker's process environment.
- [ ] **`argo setup --check` passes** against the production env.
- [ ] **Vertical lint clean** — `argo vertical lint <id>`
      against any custom verticals; fix warnings before they
      cause runtime confusion.
- [ ] **Smoke script passes** — `./scripts/smoke-skycot.sh`
      after install. 17 checks cover the operator surface +
      lint-clean across all 3 Worker v3 verticals.
- [ ] **Doctor confirms healthy** — `argo worker doctor`
      shows expected vertical loaded + zero unexpected
      failures or blocked rows in the ledger.
- [ ] **Read the demo script** — `docs/skycot/demo_script.md`
      walks the operator through a real run end-to-end before
      they touch their own data.
- [ ] **Read `failure_modes.md`** — 11 modes with recovery
      procedures so the operator isn't ad-libbing under
      pressure.
- [ ] **Approval gate test** — confirm in your own workspace
      that `argo worker actions execute` refuses an
      approval-gated action without `approve` first. The
      smoke script does this; verify locally too.
- [ ] **At least one non-mock smoke job** ran end-to-end and
      `completed_jobs.stop_reason` was NOT `need_info`.
- [ ] **Key rotation procedure tested at least once** —
      planned restart with the rotation runbook in
      `provider_keys.md` § Key rotation.
- [ ] **`/metrics` is being scraped** + an alert threshold on
      `argo_spend_usd_micros_total` is configured.

## CI status

Local verification across the worker stack:

- 80+ unit tests under `crates/argo-cli/src/handlers/{worker_actions*, worker_doctor, work_jobs, gateway/metrics_actions}` and the `worker_actions_*` sibling test files.
- 14+ integration tests under `crates/argo-cli/tests/{work_action_*, vertical_ai_receptionist, vertical_sales_follow_up, vertical_eval}` including the cohort-wide `cli_eval_worker_v3_verticals_real_assets_pass` lock.
- All authored files under the 500-LOC budget.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo test --workspace` exit 0 (modulo the pre-existing OCI port-binding flake which is documented in `MEMORY.md`).
- `./scripts/smoke-skycot.sh` 17-check end-to-end pass — covers full operator flow + lint-clean across all 3 Worker v3 verticals.

Remote CI billing on the GitHub Actions side is intermittently blocked at the account level; every PR's code is verified locally before merge as a matter of policy.

## Confidence summary

> **The OS layer is launch-ready for all three shipped verticals
> (marketplace seller, AI receptionist, sales follow-up) and
> any new vertical authored by TOML+prompt only.** The safety
> primitives that matter (approval gating, evidence gating,
> undeclared-action rejection, manifest-canonical classification,
> retry-policy enforcement, **cross-vertical isolation**) are
> enforced in code, not prompts, and locked in regression tests.
> The operator surface is coherent and documented end-to-end:
> CLI + artifact bundles + workspace ledger files + `worker
> doctor` + smoke script + `work jobs` queue view +
> `provider_keys.md` for the secrets pipeline + `failure_modes.md`
> for runbook recovery + Prometheus / JSON metrics for the
> action ledger. The launch path is gated only on what
> third-party integrations the customer wants — none of those
> are in v1's contract.
