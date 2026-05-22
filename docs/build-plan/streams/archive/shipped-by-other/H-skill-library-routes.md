# Stream H (follow-up) — Gateway routes for layered skill lookup

**Owner**: codex. **Depends on**: H-4b (#803) merged.

## Scope

HTTP endpoints exposing the H-4b `LayeredLookup` so the
worker's runtime (and the operator's review UI) can consult
the workspace + library tiers via the gateway instead of
linking against argo-skill directly.

Endpoints:
- `GET /skills?risk=low|medium|high` — list every active
  skill across both layers, filtered by trust eligibility
  for the given risk level. Response includes `layer`
  (`workspace` | `library`) per row so the UI can label.
- `GET /skills/<name>` — find a single skill by name; falls
  back to library if not in workspace. Returns 404 when
  neither layer has a match.

These are READ-ONLY endpoints. Writes still go through the
existing `SkillStore` API (out of scope here).

## Files you may touch
- `crates/argo-cli/src/handlers/gateway/routes_skills.rs`
  (extend if it exists, NEW if not — codex checks)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly
  scoped: at most ONE `pub mod routes_skills;` line + ONE
  router merge line. If `routes_skills` already exists,
  modify nothing in `mod.rs`.

## Files you must NOT touch
- `crates/argo-skill/**` — read-side wrapper only; the
  `LayeredLookup` API is fixed (don't extend).
- Any other `routes_*.rs`.

## Acceptance criteria
- [ ] `GET /skills?risk=high` returns only skills whose
      provenance grade ≥ PublishedVerified (per H-4a's
      `is_skill_eligible`).
- [ ] Each row includes `layer: "workspace" | "library"`.
- [ ] Workspace overrides library on duplicate names — the
      response only includes the workspace entry (mirrors
      H-4b's `list_for_action` behaviour).
- [ ] Missing `?risk=` defaults to `medium` (be explicit in
      the brief response so the operator knows what they
      asked for).
- [ ] Invalid risk values return 400 with a clear message.
- [ ] Integration test: install one workspace skill +
      one library skill, hit both endpoints, assert
      shapes + filters.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-skill-library-routes`
- PR title: `gateway: HTTP routes for layered skill lookup (H-skill-library-routes)`

## Notes
- The "library" store is wired as a separate `SkillStore`
  pointed at a different workspace path. For v1 the gateway
  reads from the same `SessionStore` for both — codex passes
  `LayeredLookup::workspace_only(...)` in the v1 wiring and
  notes the multi-store hookup as a TODO. Don't build the
  multi-store hookup here.
