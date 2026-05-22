# Stream E — Worker / Persona / Role primitives

**Owner**: codex. **No dependencies** (foundational).

## Scope

New `argo-identity` crate that introduces the Worker / Persona
/ Role primitives the mission requires. Today the codebase
treats a "vertical" as both template and instance; this brief
separates them.

## Files you may touch
- `crates/argo-identity/**` (NEW)
- `Cargo.toml` (workspace member only)
- `crates/argo-store/migrations/<next>__worker_identity.sql`
  (NEW migration adding tables)
- `crates/argo-cli/src/cli_worker_cmds.rs` (extend `WorkerCmd`
  enum with new `Create / Show / List` variants — do not
  modify existing `Actions`, `Items`, `Doctor` variants)
- `crates/argo-cli/src/handlers/worker_create.rs` (NEW handler
  module)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly scoped:
  add ONE line `pub mod worker_create;` (or
  `mod worker_create;` if pub is not needed). Do not modify
  any other declarations.
- `crates/argo-cli/src/handlers/worker_actions.rs` — narrowly
  scoped: add new arms to the `match cmd { ... }` dispatcher
  for the new `WorkerCmd::Create | Show | List` variants,
  routing them to `worker_create::run / show / list`. Do not
  modify any existing arms or any function in this file
  outside the dispatcher's match block.

## Files you must NOT touch
- `crates/argo-vertical/**` (templates stay as-is)
- `Executor` trait
- `argo-channels`
- Any other stream's files
- Any function in `worker_actions.rs` other than the
  dispatcher match block (e.g. do not touch `find_action`,
  helper functions, etc.)
- Any other file in `crates/argo-cli/src/handlers/` other
  than `mod.rs` (one-line add) and the new
  `worker_create.rs`

## Acceptance criteria
- [ ] Types: `Worker { id, tenant_id, vertical_id, persona,
      role, memory_namespace, auto_approval, channels }`
- [ ] Types: `Persona { name, voice_profile, avatar, bio,
      builder_attribution, provenance_chain }`
- [ ] Types: `Role { charter, duties: Vec<Duty>, kpis,
      escalation }`; `Duty { cron, entry_action, description }`
- [ ] SQLite tables: `worker`, `persona`, `role`, `duty`
- [ ] CLI: `argo worker create --vertical <id> --persona "<name>" --charter "<text>"`
      writes a Worker row + registers any cron-driven Duties
      via `argo cron`
- [ ] CLI: `argo worker show <worker-id>` prints persona +
      role + duties
- [ ] CLI: `argo worker list` lists all workers per tenant
- [ ] Memory namespace `MemoryNs` type carries `tenant_id` +
      `worker_id` + `scope` (worker_self / role / shared_team);
      every memory query MUST be parameterized by it (will be
      enforced more deeply in Stream F)
- [ ] System-prompt composition is updated to
      `Persona.voice_profile.preamble + Role.charter +
      vertical.system_prompt + memory.compose_context()`
      (Stream F will fill in `compose_context`; for now stub
      it)
- [ ] Unit + integration tests for create / show / list
- [ ] `cargo clippy -p argo-identity -- -D warnings` clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-E-identity`
- PR title: `identity: Worker / Persona / Role primitives + sqlite migration (E-identity)`

## Notes
- Worker = template (vertical) + instance metadata
  (persona/role/memory_namespace). Don't merge the concepts.
- Persona attribution: who built this worker? store the
  operator's email or handle.
- Provenance chain on persona: list of skill_ids the persona
  has consistently used (for trust signals).
