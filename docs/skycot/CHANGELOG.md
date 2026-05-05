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

## [0.1.1] — 2026-05-05 — Customer-launch hardening

This release rolls up the operational hardening that landed
post-0.1.0 (#751–#762). All 0.1.0 surface area still works the
same; this release adds the operator-facing primitives that
turn "ready to demo" into "ready for the first paying
customer."

### Added — Retry policy with `max_attempts` + force override

- `[actions.<id>.retry]` block in `vertical.toml` declares
  `max_attempts`, `backoff`, and `initial_delay_seconds`
  per action type.
- `argo worker actions retry <id>` enforces the policy by
  counting prior retries in the evidence chain. Exits non-
  zero when the next attempt would exceed `max_attempts`.
- `--force` override lets the operator bypass the cap with
  an explicit ack. The override is logged at WARN level so
  audit trails show who forced what.
- `argo worker actions show <id>` surfaces
  `attempt N/max_attempts` in both text + JSON output so the
  operator sees the budget before deciding to retry.
- gtm-engineer manifest declares retry policies on every
  declared action: 3× for verify_price, 2× for the
  approval-gated send / post / message-the-buyer family.
  (#755)

### Added — Canonical failure-modes reference

- New `docs/skycot/failure_modes.md` — 11 modes covering
  provider failures, worker crash mid-step, worker crash
  mid-execute, DB lock, DB corruption, disk full, action
  retry exhausted, action stuck pending, concurrent workers,
  vertical fails to load, and abandoning a stuck job.
- Each mode follows symptom / cause / recovery / prevention
  shape so an on-call operator can navigate the runbook
  without scanning prose. (#756)

### Added — Prometheus + JSON metrics for the action ledger

- `/metrics` (text exposition) + `/metrics/json` now expose
  three new gauges:
  - `argo_worker_actions_total{state="<state>"}` — per-state
    rows, deduped to the latest snapshot per action_id.
    Mirrors the operator's `argo worker actions list` view
    so dashboards align with the CLI.
  - `argo_worker_actions_pending_approval_total` — the
    operator's review queue depth.
  - `argo_worker_actions_pending_autonomous_total` — pending
    rows that the worker may advance without operator
    intervention.
- Read-only / scrape-safe; never mutates ledger state. (#757)

### Added — Provider keys + production secrets flow

- New `docs/skycot/provider_keys.md` — local development via
  `argo setup`, three production patterns (systemd Env /
  k8s Secret + envFrom / Vault wrapper script), what NOT to
  do, key rotation runbook, spend tracking via `/metrics`,
  and a 6-box production launch checklist. (#758)

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

### Added — Documentation refresh

- `docs/skycot/launch_readiness.md` rewritten end-to-end to
  reflect the work that landed in 0.1.1 — the "Ready" table
  grows by 6 capabilities, the "Not ready" table loses
  Prometheus metrics (shipped) and gains per-tenant spend
  caps + live-LLM eval scoring as v1.x roadmap items, and
  the pre-launch checklist grows 6 → 11 items. (#760)
- `argo work logs <job_id>` for direct artifact-bundle
  viewing without manually opening `.skycot/outputs/`. (#751)
- `examples/sales-follow-up/` — 5 lead scenarios + sample
  research / draft / approval checklist, parallel to the
  marketplace-seller and ai-receptionist examples. (#752)
- README + CHANGELOG roll-ups so all 5 shipped verticals
  show up in the docs surface. (#753)
- 14th smoke check (`argo vertical lint` across all 3 v3
  verticals) — superseded by 17 checks in 0.1.1. (#749)

### Internal — Dev runtime guardrails

- `scripts/dev-safe-check.sh` + `dev-safe-clean.sh` +
  `dev-safe-test.sh` for long autonomous coding sessions:
  pre-flight disk health gate, target/ + tempdir cleanup,
  fmt + clippy + test in one shot. Operator-invisible. (#754)

---

## Unreleased

### Added — `argo setup --check --require <ENV_VAR>` strict-pair mode

- Repeatable `--require ENV_VAR` flag turns the inclusive-OR
  contract ("at least one provider key set") into a strict-AND
  contract ("every named var must be set + non-empty"). Used to
  lock the launch-checklist invariant that BOTH primary AND
  fallback provider keys must be loadable so the fallback
  actually works on 503.
- When multiple required vars are missing, every name appears
  in the FAIL output so the operator can fix them all in one
  shell-rc edit (not re-run after each export).
- Whitespace-only env values count as missing (catches the
  `EnvironmentFile=` typo where a var is declared but blank).
- `--require` without `--check` returns a friendly error
  rather than silently triggering the interactive walkthrough.
- 5 new integration tests + the existing single-key contract
  remains green.
- `provider_keys.md` + `launch_readiness.md` updated to show
  the strict-pair invocation in the production-launch
  checklist. (#764)

### Added — Smoke check #18 covers strict-pair `--require`

- `./scripts/smoke-skycot.sh` grows from 17 → 18 checks.
- Check #18 verifies the strict-pair contract from #764:
  with `--require ANTHROPIC_API_KEY --require OPENAI_API_KEY`
  + only `ANTHROPIC_API_KEY` set, `--check` exits non-zero
  AND names `OPENAI_API_KEY` on stderr; with both set, exits
  zero with `"all required keys loadable"`. (#765)
