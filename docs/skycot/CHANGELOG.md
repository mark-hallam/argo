# SKYCOT changelog

Customer-facing release notes for the SKYCOT worker layer. The
underlying Argo platform has its own release stream tracked in
the top-level git history; this file covers ONLY changes that
matter to operators running workers + verticals.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: SemVer.

---

## [0.1.0] — 2026-05-05 — Initial customer-launch release

The first SKYCOT release ready for paying customers. Marketplace
seller (`gtm-engineer`) and AI Receptionist (`ai-receptionist`)
verticals ship in the binary; vertical authoring is a
TOML-and-prompt-only task with the OS-level safety primitives
enforced in code.

### Added — Worker v3 generic execution layer

- Vertical-declared action vocabulary via `[actions]` section
  in `vertical.toml`. Verticals own action types; the OS owns
  state machines / ledgers / approval gating / artifact
  bundles. (#729)
- Worker emits structured ` ```action ``` ` JSON blocks during
  multi-step turns. The OS classifies each emission against
  the bound vertical's manifest; undeclared types land
  `blocked`. (#729)
- Per-job artifact bundle at
  `.skycot/outputs/<job_id>/`: `transcript.md`,
  `assistant_output.md`, sectioned `research.md` /
  `pricing.md` / `listings.md` / `approval_checklist.md`,
  plus `execution_plan.md` (markdown) + `actions.json`
  (machine-readable) + `result.json` (index pointers + counts).
  (#729)

### Added — Approval CLI + state transitions

- `argo worker actions {approve, approve-batch, execute,
  complete, fail, status, render, retry}`. Each transition is
  validated against `ActionState::can_transition_to` and
  persisted as a NEW append-only memory row. (#730, #737)
- Workspace-level ledger files at `.skycot/`:
  `approval_queue.md`, `completed_actions.md`,
  `blocked_actions.md`. Auto-refreshes after every
  transition. (#730, #735)

### Added — Human-executable executor v1

- `Executor` trait + `HumanExecutor` impl. When `execute`
  fires, the executor renders a per-action artifact bundle at
  `.skycot/outputs/<job_id>/actions/<action_id>/` with
  `instructions.md` + `evidence.md` + `action.json`. The
  human carries the action out offline, then `complete`
  records the result. (#731)
- Vertical-aware instruction templates with per-platform
  overrides. `gtm-engineer`'s `post_listing` action ships
  ebay + facebook_marketplace template specialisations;
  `ai-receptionist`'s `send_calendar_invite` ships a Google
  Calendar specialisation. (#732)

### Added — Operator surface

- `argo work jobs list` — read-side queue inspector with
  state / vertical / limit / json filters. (#742)
- `argo worker doctor` — one-shot SKYCOT health check with
  vertical loading status + ledger counts + recent failures
  + workspace ledger file mtimes. (#739)
- `argo vertical lint` extended to catch `{{ key }}`
  references in templates that don't resolve to known
  top-level keys or `payload.<dotted.path>` form. (#736)

### Added — Customer launch package

- `docs/skycot/README.md` — what SKYCOT is + safety
  boundaries + read-next pointers. (#733)
- `docs/skycot/getting_started.md` — 10-minute path from
  install to first approved + executed action. (#733)
- `docs/skycot/demo_script.md` — 9-scene end-to-end
  walkthrough with talk track. (#733)
- `docs/skycot/operator_runbook.md` — full CLI reference,
  per-job artifact bundle layout, common operator flows,
  troubleshooting. (#733, #740, #744)
- `docs/skycot/launch_readiness.md` — single-page confidence
  audit. (#740, #744)
- `examples/marketplace-seller/` — 8 sample prompts +
  hand-crafted artifact bundle + 6-decision-point operator
  flow walkthrough. (#738)
- `scripts/smoke-skycot.sh` — 13-check end-to-end install
  verification (mock provider; no API keys). (#734, #740)

### Added — Vertical bindings

- `verticals/gtm-engineer/` — marketplace seller. 10 declared
  action types with per-platform instruction templates.
  (#729, #732)
- `verticals/ai-receptionist/` — front-desk worker. 6
  declared action types covering call triage, message
  capture, calendar booking, follow-up email. (#743)
- `verticals/sales-follow-up/` — warm-lead follow-up worker.
  6 declared action types covering research, drafting,
  approval-gated send / cadence / CRM logging. (#746)

### Added — Authoring + customer-facing examples

- `verticals/AUTHORING_v3_ACTIONS.md` — full walkthrough of
  the `[actions]` declaration: field reference, three
  reference action vocabularies, classification pattern, the
  safety-boundary lock test every action-emitting vertical
  should ship. (#747)
- `examples/ai-receptionist/` — 6 inbound triage prompts +
  hand-crafted artifact bundle showing receptionist-domain
  output, parallel to `examples/marketplace-seller/`. (#748)

### Safety primitives (enforced in code, not prompts)

- **Approval gate.** `requires_approval = true` rows cannot
  transition `pending → executing` without `approve` first.
  Locked in `worker_actions_transitions::execute` + tested in
  `execute_refuses_pending_approval_gated_action`.
- **Evidence gate.** When the worker's research evidence is
  insufficient (zero web_search hits, sub-200-char extracts),
  approval-gated emissions are downgraded to `blocked`
  automatically. Locked in
  `emit_orchestrator_blocks_approval_action_when_evidence_insufficient`.
- **Capability-grant rejection.** Emissions of
  `action_type`s not declared in the bound vertical's
  manifest land `blocked` with a reason naming the gap.
- **Manifest-canonical classification.** `risk_level` and
  `requires_approval` are stamped from the manifest onto
  every persisted row. The model cannot override.
- **Cross-vertical isolation.** Actions emitted under
  vertical A are NEVER visible to commands scoped to vertical
  B. Locked end-to-end in
  `actions_emitted_under_gtm_engineer_are_invisible_to_demo_shop`. (#741)
- **Manifest classification locks for every shipped vertical.**
  gtm-engineer + ai-receptionist + sales-follow-up each ship a
  regression test pinning their action-classification table.
  A drift means an approval-gated action silently became
  autonomous. (#728, #743, #746)

### Test coverage

- 80+ unit tests across the worker stack.
- 14+ integration tests driving the actual `argo` binary
  end-to-end (transition flows, retry, isolation, ai-receptionist
  + sales-follow-up manifest locks).
- 14-check smoke script covering install verification +
  lint-clean across all 3 Worker v3 verticals. (#749)
- `cargo fmt --all` + `cargo clippy --workspace
  --all-targets -- -D warnings` clean across the workspace.

### What's NOT in v1

Decision-gated, not blocked. See `launch_readiness.md` for the
full table. Headlines:

- Browser-automation executor (depends on browser MCP fully
  wired).
- Real marketplace SDK executors (no vendor APIs in v1).
- Native voice TTS (waiting on `piper-rs` upstream).
- Prometheus metrics for the action ledger
  (`/metrics` exposes Argo-platform metrics today; action
  counters TBD).
- Multi-vertical deployment isolation hardening (memory rows
  are scoped per-vertical; no explicit cross-tenant RBAC
  layer beyond what Argo's existing tenancy provides).

---

## Unreleased

### Added — Eval suite expansion across the Worker v3 cohort

- `verticals/gtm-engineer/eval/worker_v3_emission.jsonl`: 7
  fixtures covering action-block emission, the evidence-gate,
  undeclared-action rejection, the autonomous-safe / approval
  split, and retry-chain evidence-carry. Adds to the existing
  10-fixture `gtm_engineer_core.jsonl` for 17 total.
- `verticals/ai-receptionist/eval/triage_core.jsonl`: 7
  fixtures covering inbound triage (sales, recruiter, spam,
  ambiguous, urgent), approval-gated calendar invites, and
  undeclared-action rejection.
- `verticals/sales-follow-up/eval/followup_core.jsonl`: 7
  fixtures covering signal-led drafting (post-demo, post-trial,
  role-change, podcast), the abstain contract on dropped
  leads, approval-gated email send, autonomous-safe CRM logging.
- New `[evaluation]` sections wired into `ai-receptionist` +
  `sales-follow-up` `vertical.toml` files (gtm-engineer already
  had one; new suite added to its list).
- CI integration test
  `cli_eval_worker_v3_verticals_real_assets_pass` locks all
  three suites to a minimum-fixture floor — a future "rip out
  fixtures" PR fails CI loudly.
- AUTHORING_v3_ACTIONS.md gains an "Eval suites" section
  documenting the rule keys + the cohort's structural
  coverage. (#759)

### Added — Smoke script extensions

- `./scripts/smoke-skycot.sh` grows from 14 → 17 checks.
- Check #15: every Worker v3 vertical's eval suite passes
  `argo vertical eval --json` and carries ≥5 fixtures.
- Check #16: `argo setup --check` exits non-zero with a
  `[FAIL]` marker when no provider env vars are exported.
- Check #17: `argo setup --check` exits zero with `[ok]`
  when ANTHROPIC_API_KEY is set. (#762)

### Added — `argo setup --check` for launch checklists

- New `--check` mode on `argo setup` skips the interactive
  walkthrough and verifies that at least one provider key
  (ANTHROPIC / OPENAI / GEMINI / AWS_ACCESS_KEY_ID /
  ELEVENLABS) is loadable from the process env. Exits 0 on
  success, non-zero with a clear FAIL message + canonical
  resolution-order summary on failure. Designed for CI and
  the production-launch checklist in `provider_keys.md`.
- Rejects empty / whitespace-only env values so a broken
  `EnvironmentFile=` doesn't silently look configured.
- 4 integration tests + 1 unit test lock the contract.
- `provider_keys.md` aligned with reality: removed
  aspirational claims about `argo setup` writing a
  `~/.config/argo/secrets.toml` (the resolver only reads
  env today; secrets.toml is a v1.x roadmap item). (#761)
