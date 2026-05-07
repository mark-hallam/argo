# claude-tracing-span-naming-consistency — canonical span hierarchy + workspace rewrite

**Owner**: claude. **Depends on**: nothing. **Phase B observability narrow.**

## Why

Tracing spans across the workspace use **three different naming conventions** mixed at random:

- `session_create` (snake_case verb-noun)
- `gateway.session.create` (dotted hierarchical, frontend-mode)
- `create_session` (snake_case noun-verb, opposite order)

Trace dashboards show duplicate clusters. Filtering by span name requires three OR clauses. New contributors copy-paste from whichever neighbouring file they happen to land on. The codebase has ~80+ `span!`/`tracing::span!`/`#[instrument]` sites and the drift is worsening.

This brief defines + documents the canonical hierarchy and rewrites every site to match.

## Approach (concrete)

**Two-phase: standard + rewrite.**

### Phase 1 — define canonical hierarchy (write once)

1. **Top-level domains** (first dotted segment): `gateway`, `cli`, `agent`, `worker`, `store`, `provider`, `channels`, `marketplace`, `vertical`, `memory`, `skill`, `forge`, `tui`, `mcp`.
2. **Sub-domain** (second segment): the noun (`session`, `message`, `route`, `manifest`, `action`, `event`, etc.).
3. **Action verb** (third segment, if present): `create`, `read`, `list`, `update`, `delete`, `dispatch`, `validate`, `resolve`, `emit`, `consume`.
4. **Format**: `domain.subdomain.action` (dotted, lowercase, snake_case within segments).
5. **Examples**:
   - `gateway.session.create` (POST /sessions)
   - `gateway.message.list` (GET /sessions/:id/messages)
   - `agent.turn.execute` (one turn of the agent loop)
   - `worker.action.dispatch` (worker dispatches an action)
   - `store.session.read` (DB read for a session)
   - `provider.anthropic.complete` (model call)
6. Document the contract in `docs/skycot/TRACING_SPAN_NAMING.md` (NEW, ≤ 150 LOC).

### Phase 2 — rewrite every site

1. `rg 'tracing::span!|tracing::info_span!|tracing::debug_span!|tracing::trace_span!|#\[instrument' --type rust > /tmp/span-sites.txt`. Expect ~80 sites.
2. For each site: rename the span to `domain.subdomain.action` form. Preserve all fields/attributes verbatim. Where the existing name is ambiguous, infer from surrounding context + the file path.
3. Rewrite `#[instrument(name = "...")]` overrides to match.
4. Where `#[instrument]` has no explicit name (uses fn name as default): add explicit `name = "domain.subdomain.action"` if the fn name doesn't already match.

## Files you may touch

- `docs/skycot/TRACING_SPAN_NAMING.md` (NEW, ≤ 150 LOC)
- Every Rust file in the workspace containing a `span!` / `instrument` call — narrow rename only, no logic change
- `scripts/check-span-naming.sh` (NEW, ≤ 80 LOC) — CI heuristic that flags spans not matching `^[a-z_]+\.[a-z_]+(\.[a-z_]+)?$`
- `.github/workflows/*.yml` (existing CI workflow) — narrowly: add 1 step invoking the new script

## Files you must NOT touch

- Span fields / structured-log keys (separate brief if needed)
- Log macros (`info!`, `warn!`, `error!`) — only span constructors are in scope
- Any test asserting on a specific span name (those tests must be updated only via the rename, no logic change)

## Acceptance criteria

- [ ] `docs/skycot/TRACING_SPAN_NAMING.md` exists + documents the contract
- [ ] Every span in the workspace matches the `domain.subdomain.action` regex
- [ ] `scripts/check-span-naming.sh` runs in CI and is green
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] Every modified file ≤ 500 LOC
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-tracing-span-naming-consistency`
- PR title: `tracing: canonical span hierarchy + workspace rewrite (claude-tracing-span-naming-consistency)`

## Notes for the implementer

- The rewrite is **mechanical-but-large**. Budget time to do it in 5-10 commits along domain boundaries (one commit per top-level domain) so review can chunk it.
- A handful of spans don't fit cleanly (e.g. background reaper loops). Use `domain.subdomain.tick` or `domain.subdomain.loop` for those — document the convention in the new doc.
- The CI check should be regex-based; full AST parse is overkill.
- If you encounter a span that obviously belongs to a domain that isn't on the top-level list above, ADD it to the doc (don't invent a new format).
- Trace dashboards consuming the old names will break — flag this in the PR description so dashboard owners can update.
