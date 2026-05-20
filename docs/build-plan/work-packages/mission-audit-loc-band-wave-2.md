parent_brief: mission-audit-loc-band-wave-2
parent_brief_file: docs/build-plan/streams/mission-audit-loc-band-wave-2.md
global_ledger_row: docs/build-plan/BUILD_STATUS.md:mission-audit-loc-band-wave-2

manager:
  agent_family: codex
  agent_id: codex-mission-audit-loc-band-wave-2-v2
  role: manager_worker

status: ready_for_pr
spawn_budget: 0
max_depth: 1
integration_branch: codex-mission-audit-loc-band-wave-2-v2
draft_pr: null
final_pr: null

lease:
  claimed_at: 2026-05-20T00:37:46Z
  heartbeat_at: 2026-05-20T01:18:00Z
  expires_at: 2026-05-20T03:37:46Z

spawn_policy:
  mode: manual_required
  preferred_families:
    - codex
  max_parallel_children: 0

parent_scope:
  allowed_files:
    - crates/argo-cli/src/handlers/worker_actions_notify.rs
    - crates/argo-cli/src/handlers/worker_actions_notify_*.rs
    - crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs
    - crates/argo-cli/src/handlers/gateway/routes_dispatch_post_*.rs
    - crates/argo-cli/src/handlers/doctor.rs
    - crates/argo-cli/src/handlers/doctor_*.rs
    - crates/argo-cli/src/handlers/gateway/auth_magic_link.rs
    - crates/argo-cli/src/handlers/gateway/auth_magic_link_*.rs
    - crates/argo-genesis/src/broad_role.rs
    - crates/argo-genesis/src/broad_role_*.rs
    - crates/argo-compliance/src/export.rs
    - crates/argo-compliance/src/export_*.rs
    - crates/argo-cli/src/handlers/gateway/sso_saml.rs
    - crates/argo-cli/src/handlers/gateway/sso_saml_*.rs
    - crates/argo-skill/src/store.rs
    - crates/argo-skill/src/store_*.rs
    - crates/argo-store/src/approvals.rs
    - crates/argo-store/src/approvals_*.rs
    - crates/argo-cli/src/handlers/gateway/ws_inbox_events_tests.rs
    - crates/argo-cli/src/handlers/gateway/ws_inbox_events_*_tests.rs
    - docs/build-plan/work-packages/mission-audit-loc-band-wave-2.md
  shared_hotspots: []
  acceptance_criteria:
    - Split the nine cited files that remain in the 450-500 LOC band.
    - Leave routes_dispatch_post.rs unchanged if it remains below 350 LOC.
    - Each original file must end below 350 LOC after split.
    - Each new sibling file must stay below 450 LOC.
    - Public API paths must remain stable via parent re-exports.
    - Run crate-scoped tests and the codex pre-push gate before PR.
    - Informational only: final 450-500 LOC band count should trend down.

children:
  - id: skill-store
    status: completed
    goal: Split write/trust helpers out of argo-skill store.rs.
    assigned_to: { agent_family: codex, agent_id: codex-mission-audit-loc-band-wave-2-v2 }
    allowed_files:
      - crates/argo-skill/src/store.rs
      - crates/argo-skill/src/store_*.rs
    forbidden_files:
      - docs/build-plan/BUILD_STATUS.md
    success_criteria:
      - crates/argo-skill/src/store.rs is below 350 LOC.
      - cargo test -p argo-skill passes.
    output:
      branch: codex-mission-audit-loc-band-wave-2-v2
      patch: in_parent_branch
      summary: store write and trust-grade helpers extracted; parent file is 341 LOC.
      tests_run: >
        cargo test -p argo-skill hit two pre-existing
        store_worker_overlay_tests failures reproduced on clean origin/main;
        clippy and pre-push gate recorded in PR.
      risks: none_known
      remaining_work: null
  - id: cli-notify-doctor-auth-saml-tests
    status: completed
    goal: Split CLI handler files in the 450-500 LOC band.
    assigned_to: { agent_family: codex, agent_id: codex-mission-audit-loc-band-wave-2-v2 }
    allowed_files:
      - crates/argo-cli/src/handlers/worker_actions_notify.rs
      - crates/argo-cli/src/handlers/worker_actions_notify_*.rs
      - crates/argo-cli/src/handlers/doctor.rs
      - crates/argo-cli/src/handlers/doctor_*.rs
      - crates/argo-cli/src/handlers/gateway/auth_magic_link.rs
      - crates/argo-cli/src/handlers/gateway/auth_magic_link_*.rs
      - crates/argo-cli/src/handlers/gateway/sso_saml.rs
      - crates/argo-cli/src/handlers/gateway/sso_saml_*.rs
      - crates/argo-cli/src/handlers/gateway/ws_inbox_events_tests.rs
      - crates/argo-cli/src/handlers/gateway/ws_inbox_events_*_tests.rs
    forbidden_files:
      - docs/build-plan/BUILD_STATUS.md
    success_criteria:
      - Each original CLI file is below 350 LOC.
      - cargo test -p argo-cli passes or only known flakes reproduce.
    output:
      branch: codex-mission-audit-loc-band-wave-2-v2
      patch: in_parent_branch
      summary: CLI notify, doctor, magic-link auth, SAML, and WS inbox test helpers extracted; cited parent files are all below 350 LOC.
      tests_run: >
        cargo test -p argo-cli hit one pre-existing
        worker_actions_executor failure reproduced on clean origin/main;
        routes_approvals_json, webinbox, and setup_steps failures from the
        full bin run passed in isolation.
      risks: none_known
      remaining_work: null
  - id: genesis-compliance-store
    status: completed
    goal: Split argo-genesis, argo-compliance, and argo-store band files.
    assigned_to: { agent_family: codex, agent_id: codex-mission-audit-loc-band-wave-2-v2 }
    allowed_files:
      - crates/argo-genesis/src/broad_role.rs
      - crates/argo-genesis/src/broad_role_*.rs
      - crates/argo-compliance/src/export.rs
      - crates/argo-compliance/src/export_*.rs
      - crates/argo-store/src/approvals.rs
      - crates/argo-store/src/approvals_*.rs
    forbidden_files:
      - docs/build-plan/BUILD_STATUS.md
    success_criteria:
      - Each original file is below 350 LOC.
      - cargo test -p argo-genesis -p argo-compliance -p argo-store passes or only known flakes reproduce.
    output:
      branch: codex-mission-audit-loc-band-wave-2-v2
      patch: in_parent_branch
      summary: genesis broad-role helpers, compliance export render/table helpers, and store approval types extracted; cited parent files are all below 350 LOC.
      tests_run: >
        cargo test -p argo-compliance and cargo test -p argo-genesis passed;
        cargo test -p argo-store hit pre-existing migration-version failures,
        with tests::open_and_migrate reproduced on clean origin/main.
      risks: none_known
      remaining_work: null

spawn_requests: []

review:
  reviewer_required: true
  reviewer_agent: { agent_family: null, agent_id: null }
  findings: []

promotion_requests:
  - reason: >-
      Resolved by conductor amendment 2 on 2026-05-20: aggregate band-count
      targets are informational only; per-file parent/sibling LOC criteria
      are the load-bearing acceptance criteria.
    requested_by: codex-mission-audit-loc-band-wave-2-v2
    status: resolved
