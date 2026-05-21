# mission-knowledge-context-injection — inject workplace wiki contents into every worker turn's system message

**Owner**: claude. **LOAD-BEARING brief — this is what makes the wiki real, not theater.**
**Depends on**: `mission-workplace-knowledge-substrate` MERGED (provides `argo_workplace_knowledge::doc::list` + `max_version_for_tenant`) + `mission-workplace-knowledge-audit-kinds` MERGED + `mission-workplace-knowledge-gateway` MERGED (CRUD is real).
**Authored against**: origin/main `c613d922` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-knowledge-context-injection`.
**Priority**: **P0** — Wave A brief 5 of 6 from `docs/skycot/design/workplace-onboarding-architecture.md`. The wiki is theater without this brief.
**Sizing**: ~700 LOC across system-prompt extension + cache + debug-emission + integration tests.

## Why this brief exists

Read design doc §1 (vision), §2 (Pillar 1), §3 system-message shape, §4 "Knowledge-doc context injection at turn-start" (one of the 3 genuinely-new things being built), §5 brief 5 (this spec), §7 Pillar 1 acceptance criteria (especially items 2 + 3 — debug-mode visibility + real LLM behavior change).

The substrate (brief 1) + endpoints (brief 3) + editor (brief 4) let operators write to the wiki. **But none of that reaches the model.** Brief 5 wires the wiki contents into every worker turn's system message, so:

> Operator edits brand-voice doc to "never use 'cheap'" → save → next worker turn that touches pricing → the model output avoids "cheap" because the doc body is literally in the system prompt the LLM saw.

This is the load-bearing brick. Per the operator's directive: "make sure nothing is stubs or theatre." This brief produces the **observable**, **testable**, **provider-agnostic** wire that turns wiki edits into model behavior.

## Approach (concrete)

### Phase 0 — Find the actual system-message constructor

The system-prompt construction lives in `crates/argo-agent-runner/src/turn_system_prompt.rs` (verified at authoring time via `rg 'system_message\|system_prompt' crates/argo-agent-runner/`). Read this file end-to-end before authoring. There are sibling files that contribute context layers:

- `turn_helpers_context.rs` — context-window assembly.
- `turn_helpers_overlay.rs` — overlay layers (cohort, memory).
- `turn_memory.rs` — per-worker memory blocks.

The injection point is wherever `turn_system_prompt.rs` builds the final `system: String` field passed to the provider. The injection MUST happen at turn-start, BEFORE any per-turn user message is appended, and it MUST be cacheable.

### Phase 1 — Extend the system-prompt assembler

1. **MODIFY** `crates/argo-agent-runner/src/turn_system_prompt.rs` — add a new section to the system-prompt assembly. Position-wise, the design doc §3 system-message shape is:

   ```
   [PERSONA — from worker.persona]
   [WORKPLACE KNOWLEDGE — read-only, do not contradict]    ← NEW (this brief)
   ## Brand Voice (v3, last updated 2026-05-21)
   <body markdown>
   ---
   ## Business Rules (v1, ...)
   <body markdown>
   ---
   [END WORKPLACE KNOWLEDGE]
   [COHORT PLAYBOOK — Wave B]                              ← left as a HOOK; not implemented in Wave A
   [WORKER MEMORY — Maya]
   [WORKFLOW CONTEXT]
   [TURN INPUT]
   ```

   **Key design decisions:**
   - **Single labeled block with clear EOF marker** (the `[END WORKPLACE KNOWLEDGE]` line). This makes the cohort layer (brief #10 in Wave B) able to slot in immediately AFTER without colliding with workplace docs.
   - **No mid-turn refresh**: the injection reads the wiki ONCE at turn-start. If admin edits a doc mid-turn, the worker in flight uses the snapshot from turn-start. The next turn picks up the new content. (Per design doc §8 R2.)
   - **Doc ordering**: alphabetical-by-category, then alphabetical-by-slug within category. Stable order → cacheable.

2. **NEW** `crates/argo-agent-runner/src/turn_helpers_workplace.rs` (~250 LOC) — the workplace-knowledge fetch + format module:

   ```rust
   use argo_store::SessionStore;
   use argo_types::{ArgoResult, TenantId};
   use argo_workplace_knowledge::doc::{self, WorkplaceKnowledgeDoc, Category};
   use std::sync::Arc;
   use std::time::Duration;
   use tokio::sync::RwLock;
   use chrono::Utc;
   use std::collections::HashMap;

   /// Cache entry: keyed on (tenant_id, max_version). Invalidated when
   /// max_version moves. Hot path: one COUNT query (max_version) per turn.
   #[derive(Clone)]
   struct CacheEntry {
       tenant_id: String,
       max_version: i64,
       rendered_block: String,
       built_at: chrono::DateTime<Utc>,
   }

   /// Global per-process cache. Tenant_id keys.
   static WORKPLACE_KNOWLEDGE_CACHE: once_cell::sync::Lazy<RwLock<HashMap<String, CacheEntry>>> =
       once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

   /// Returns the rendered `[WORKPLACE KNOWLEDGE — ...]` block for the
   /// given tenant. Caches keyed on max_version so repeated turns within
   /// a few seconds reuse the same string.
   ///
   /// Returns `Ok(None)` if the tenant has no workplace docs (the section
   /// is omitted entirely from the system prompt).
   pub async fn render_workplace_knowledge_block(
       store: Arc<SessionStore>,
       tenant_id: &TenantId,
   ) -> ArgoResult<Option<String>> {
       // 1. COUNT-only query — cheap.
       let current_max = doc::max_version_for_tenant(store.pool(), tenant_id).await?;
       if current_max == 0 { return Ok(None); }

       // 2. Cache lookup.
       {
           let cache = WORKPLACE_KNOWLEDGE_CACHE.read().await;
           if let Some(entry) = cache.get(tenant_id.as_str()) {
               if entry.max_version == current_max {
                   return Ok(Some(entry.rendered_block.clone()));
               }
           }
       }

       // 3. Miss → re-fetch all docs and render.
       let docs = doc::list(store.pool(), tenant_id).await?;
       let rendered = render_block(&docs);

       // 4. Cache write.
       {
           let mut cache = WORKPLACE_KNOWLEDGE_CACHE.write().await;
           cache.insert(tenant_id.as_str().to_string(), CacheEntry {
               tenant_id: tenant_id.as_str().to_string(),
               max_version: current_max,
               rendered_block: rendered.clone(),
               built_at: Utc::now(),
           });
       }

       Ok(Some(rendered))
   }

   /// Pure function — sort docs alphabetically by (category, slug), render
   /// with markdown headings + EOF marker.
   fn render_block(docs: &[WorkplaceKnowledgeDoc]) -> String {
       let mut sorted = docs.to_vec();
       sorted.sort_by(|a, b| {
           a.category.as_db_str().cmp(b.category.as_db_str())
               .then_with(|| a.slug.cmp(&b.slug))
       });

       let mut out = String::from("[WORKPLACE KNOWLEDGE — read-only, do not contradict]\n\n");
       for doc in &sorted {
           out.push_str(&format!(
               "## {} (v{}, last updated {})\n{}\n\n---\n\n",
               doc.title,
               doc.version,
               iso_date_from_micros(doc.updated_at_micros),
               doc.body_markdown.trim_end()
           ));
       }
       out.push_str("[END WORKPLACE KNOWLEDGE]\n");
       out
   }

   fn iso_date_from_micros(micros: i64) -> String { /* short YYYY-MM-DD */ }

   /// Cache invalidation hook — call from the gateway's update/delete
   /// handlers OR rely on the version-keyed lookup to self-invalidate.
   /// Default: version-key self-invalidation (no explicit invalidate call).
   pub async fn invalidate_cache_for(tenant_id: &TenantId) {
       let mut cache = WORKPLACE_KNOWLEDGE_CACHE.write().await;
       cache.remove(tenant_id.as_str());
   }

   #[cfg(test)]
   #[path = "turn_helpers_workplace_tests.rs"]
   mod tests;
   ```

3. **MODIFY** `turn_system_prompt.rs` — call `render_workplace_knowledge_block(store, &turn.tenant_id)` and inject the returned string after the persona block, before the cohort/memory layers. If `Ok(None)` (tenant has no docs), skip the block entirely.

### Phase 2 — Debug-mode emission

4. **MODIFY** `turn_system_prompt.rs` (or `turn_run_loop.rs` — wherever the constructed `system: String` becomes the provider call) — add a debug-emission gate:

   ```rust
   if std::env::var("ARGO_AGENT_DEBUG_SYSTEM_MESSAGE").as_deref() == Ok("1") {
       tracing::info!(
           target: "argo::system_message::debug",
           tenant_id = %tenant_id,
           worker_id = %worker_id,
           length_chars = system_message.len(),
           "\n{system_message}\n"
       );
   }
   ```

   This is the operator-visible proof that the injection is real. Operators set `ARGO_AGENT_DEBUG_SYSTEM_MESSAGE=1` in their gateway environment; every turn-start logs the LITERAL system message the LLM saw.

### Phase 3 — Tests

5. **NEW** `crates/argo-agent-runner/src/turn_helpers_workplace_tests.rs` (~200 LOC) — unit tests for the rendering + cache:
   - `render_empty_tenant_returns_none`.
   - `render_single_doc_includes_title_version_body`.
   - `render_multiple_docs_alphabetical_by_category_then_slug`.
   - `render_block_starts_and_ends_with_markers`.
   - `cache_hit_after_warmup` — call render twice, second call doesn't re-query the body table.
   - `cache_invalidates_on_version_bump` — write a new doc → cache miss → re-render.
   - `cross_tenant_isolation` — tenant A's cache entry doesn't appear in tenant B's render.

6. **NEW** `crates/argo-agent-runner/tests/workplace_knowledge_injection_smoke.rs` (~150 LOC) — integration test that proves the wire from doc to system message:
   - Seed a workplace doc with body `"## Voice\nNever say cheap."`.
   - Construct a turn for a worker in that tenant.
   - Invoke `turn_system_prompt::build(...)`.
   - Assert the returned system message **contains the literal string `Never say cheap.`** byte-identical to the stored doc.
   - This is the "not theater" smoke test — design doc §7 Pillar 1 acceptance #2.

7. **NEW** `crates/argo-agent-runner/tests/workplace_knowledge_debug_emit_smoke.rs` (~80 LOC) — verifies the debug-emission path:
   - Set `ARGO_AGENT_DEBUG_SYSTEM_MESSAGE=1` in the test process.
   - Run a turn-build.
   - Capture `tracing` output (via `tracing_subscriber::fmt::test`).
   - Assert the log line at `argo::system_message::debug` target appears AND contains the workplace block.

### Phase 4 — Cohort-layer hook (for Wave B brief #10)

8. **MODIFY** `turn_system_prompt.rs` — leave a clear injection point AFTER the `[END WORKPLACE KNOWLEDGE]` marker where Wave B brief 10 can drop the cohort block. Inline comment: `// COHORT PLAYBOOK injection point — see mission-knowledge-context-cohort-overlay (Wave B brief 10)`. Don't pre-emptively call any cohort code; just leave the hook.

## Files you may touch (verified to exist on `c613d922`)

- **NEW** `crates/argo-agent-runner/src/turn_helpers_workplace.rs`.
- **NEW** `crates/argo-agent-runner/src/turn_helpers_workplace_tests.rs`.
- **NEW** `crates/argo-agent-runner/tests/workplace_knowledge_injection_smoke.rs`.
- **NEW** `crates/argo-agent-runner/tests/workplace_knowledge_debug_emit_smoke.rs`.
- **MODIFY** `crates/argo-agent-runner/src/turn_system_prompt.rs` — add the block injection + debug emission. Keep file < 500 LOC.
- **MODIFY** `crates/argo-agent-runner/src/lib.rs` — register the new module (`mod turn_helpers_workplace;`).
- **MODIFY** `crates/argo-agent-runner/Cargo.toml` — add `argo-workplace-knowledge = { path = "../argo-workplace-knowledge" }` + `once_cell = { workspace = true }` (if not already present).

## Files you must NOT touch

- `crates/argo-workplace-knowledge/*` — substrate is brief 1's domain.
- `crates/argo-cli/src/handlers/gateway/*` — gateway is brief 3's domain.
- Provider adapters (`crates/argo-provider-*`) — the system message is constructed in argo-agent-runner; the providers just receive it. Don't modify the providers.
- `crates/argo-memory/*` — per-worker memory is its own layer; this brief adds a layer ABOVE it, doesn't change it.
- `crates/argo-agent-runner/src/turn_memory.rs` — leave the worker-memory composition alone; only the system-prompt assembly is in scope.

## Pre-flight verification

```bash
# Find the actual constructor — what we found at authoring time:
rg -ln 'system_message|system_prompt' crates/argo-agent-runner/src/
ls crates/argo-agent-runner/src/turn_system_prompt.rs

# Confirm the substrate is on main.
ls crates/argo-workplace-knowledge/src/lib.rs
rg 'pub async fn max_version_for_tenant|pub async fn list' crates/argo-workplace-knowledge/src/doc.rs
# Both must return matches — otherwise wait for brief 1 to merge.

# Confirm AuditKind variants exist (we don't emit them from here, but their
# existence proves brief 2 has landed; if brief 3 needs to invalidate this
# brief's cache, that's a Wave-B follow-up).
grep -n 'WorkplaceKnowledgeDocUpdated' crates/argo-audit/src/event.rs

# Confirm tracing infra in agent-runner.
rg 'use tracing::' crates/argo-agent-runner/src/turn_system_prompt.rs | head -3

# Confirm tenant_id reaches the constructor.
rg 'tenant_id' crates/argo-agent-runner/src/turn_system_prompt.rs | head -5
```

## Acceptance criteria

This brief carries the operator-locked "not theater" acceptance from design doc §7 Pillar 1. Each MUST pass:

- [ ] **Context injection observable in debug mode**: `ARGO_AGENT_DEBUG_SYSTEM_MESSAGE=1` → turn-start logs the literal system message at `argo::system_message::debug` target; the workplace block appears inline.
- [ ] **Byte-identical doc body in system message**: integration test seeds a doc with body `"Never say cheap."` and asserts the LITERAL bytes appear in the returned system message.
- [ ] **Real LLM behavior change**: the integration test does NOT need to invoke a real LLM — the smoke test only verifies the system message contains the doc body. A real-LLM verification belongs in the smoke-skycot.sh suite (sibling brief if needed); this brief proves the wire is real, not that any specific LLM respects it.
- [ ] **Audit-row-per-edit still works**: this brief doesn't touch audit emission; brief 3's audit-on-update path is independent. Verify via existing brief-3 tests, not this one.
- [ ] **No mock fallback in production**: the agent-runner reads from the SessionStore, full stop. No `if cfg!(test)` shortcut, no env-flag bypass.
- [ ] **Cache keyed on (tenant_id, max_version)** so hot path is 1 query per turn.
- [ ] **Empty tenant returns None** → the system message omits the section entirely (no empty `[WORKPLACE KNOWLEDGE]` block with nothing inside).
- [ ] **Cross-tenant isolation**: tenant A's cache entry is not visible to tenant B's render.
- [ ] **Cohort-layer hook left in place** with a comment pointing to Wave B brief 10.
- [ ] `cargo test -p argo-agent-runner --lib turn_helpers_workplace` green.
- [ ] `cargo test -p argo-agent-runner --test workplace_knowledge_injection_smoke` green.
- [ ] `cargo test -p argo-agent-runner --test workplace_knowledge_debug_emit_smoke` green.
- [ ] `cargo clippy -p argo-agent-runner -p argo-workplace-knowledge --all-targets -- -D warnings` clean.
- [ ] `bash scripts/conductor-pre-merge-gate.sh` PASS — Tier-3 review applies (this is core runtime code).
- [ ] Every modified file < 500 LOC.
- [ ] No `unwrap` / `expect` outside `#[cfg(test)]`.
- [ ] No new `pub struct` / `pub enum` / `pub type` with a name that already exists. Verify with `rg 'pub (struct|enum|type) (CacheEntry|render_block|render_workplace_knowledge_block)' crates/`.
- [ ] BUILD_STATUS.md row flipped to IN_REVIEW with PR number on push.

## Branch + PR

- Branch: `claude-mission-knowledge-context-injection`
- PR title: `argo-agent-runner: inject workplace wiki into turn-start system message (mission-knowledge-context-injection)`
- PR body MUST:
  - Call out this is **the load-bearing brick** of Wave A.
  - Include a screenshot or text excerpt of a debug-mode log line showing the injection working.
  - Claim crate-scoped + workspace clippy.

## Gotchas

- **System-prompt constructor location verified**: at authoring time it's `turn_system_prompt.rs`. If the file has been refactored (renamed or split) by the time this brief is claimed, the worker MUST grep for the actual constructor before authoring. Imagined paths cause BLOCKED claims per DRIFT_PREVENTION.md "grep before write".
- **Templated system-prompt risk**: if the system-prompt constructor uses minijinja / handlebars / tera (the operator's handoff warned about this), this brief needs to extend the TEMPLATE, not the Rust call site. Verify with `rg 'minijinja|handlebars|tera' crates/argo-agent-runner/` at claim time. If templating is in play, the brief still applies — just inject the `[WORKPLACE KNOWLEDGE — ...]` block via a template variable instead of string concat. **At authoring time pre-flight returned no template-engine matches** but the worker MUST re-verify.
- **Context-size growth**: 5 starter docs × ~2KB = 10KB. Plus cohort (Wave B) ~5KB. Plus per-worker memory ~3KB. Plus per-task ~15-50KB. Comfortable margin against Claude 200k. Design doc §8 R1 documents this. If a tenant grows to 100+ docs and crosses 50KB the cache stays valid; the worry is provider-cost, not correctness. A future brief will add relevance pruning (out of scope here).
- **Cache invalidation race**: the gateway POST/PUT/DELETE in brief 3 changes the table. The cache here keys on `max_version`. Within a single process, the next turn's COUNT query picks up the new max → cache miss → re-render. Across processes (multi-tenant gateway with multiple workers running in different binaries), the version-key approach is correct: each process invalidates on its own version drift. **Do NOT add an inter-process cache bus** for this brief — that's premature.
- **Doc ordering must be stable**: the test `render_multiple_docs_alphabetical_by_category_then_slug` enforces this. If a worker can't guarantee stable ordering, the cache key is meaningless and the LLM sees different system messages on consecutive turns. Use the `.sort_by` shown above.
- **`once_cell::sync::Lazy<RwLock<...>>` global**: SKYCOT's invariant is "no global mutable state — AgentDeps is the only injection point" (per `CLAUDE.md`). This brief introduces a process-local cache via `once_cell` which is technically global mutable state. **Justification**: it's a pure performance cache (correctness identical with cache size = 0). Acceptable in this context. If the reviewer disagrees, refactor to thread the cache via `AgentDeps` — adds ~50 LOC.

## References

- Design doc: `docs/skycot/design/workplace-onboarding-architecture.md` §1, §2, §3 (system-message shape), §4 ("Knowledge-doc context injection at turn-start"), §5 brief 5, §7 Pillar 1 (acceptance), §8 R2 (mid-turn refresh risk + mitigation).
- Sibling briefs in Wave A:
  - **brief 1** (`mission-workplace-knowledge-substrate`) — provides `argo_workplace_knowledge::doc::{list, max_version_for_tenant}`.
  - **brief 3** (`mission-workplace-knowledge-gateway`) — provides the write surface that triggers cache invalidation via version bumps.
  - **brief 6** (`mission-knowledge-startup-seed`) — provides the 5 starter docs that this brief renders on first turn.
- Wave B teaser:
  - **brief 10** (`mission-knowledge-context-cohort-overlay`) — slots in AFTER this brief's `[END WORKPLACE KNOWLEDGE]` marker; leave the hook.
- Reference patterns:
  - `crates/argo-cli/src/handlers/gateway/approval_expiry_sweeper.rs` — `once_cell::sync::Lazy<RwLock<HashMap>>` cache pattern (similar shape).
  - `crates/argo-agent-runner/src/turn_memory.rs` — adjacent context-injection layer (worker memory). Read for the shape of the per-turn composition.
