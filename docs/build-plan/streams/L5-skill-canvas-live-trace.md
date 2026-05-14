# L5 — `skill-canvas-live-trace` (Track L Phase 1, Tier 2)

**Owner**: claude. **Depends on**: L4 MERGED; E1-f WebSocket infra MERGED (shipped); L2 MERGED.
**Authored against**: origin/main `0ec56ca5` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L5-skill-canvas-live-trace`.

## Scope

Live-trace overlay via WebSocket — canvas shows the runtime's current node pulsing during execution. Tied to existing replay-sandbox infra (E5-d). LOC ~700. Branch: `claude-L5-skill-canvas-live-trace`. PR title: `feat(web/canvas): live-trace overlay via WebSocket (L5)`.

**Specifics (verbatim from L0 spec)**:

- Runtime emits an event per node enter/complete/fail.
- WS topic: `skill-trace.<skill_id>.<run_id>`.
- Canvas subscribes when viewing a live run; pulses the current node.
- Completed nodes get a green check; failed nodes red X.
- Decoupled from L4 viewer — L4 works without L5.

## Files you may touch

- NEW `crates/argo-cli/src/handlers/gateway/ws_skill_trace.rs` — WS handler emitting `node.entered` / `node.completed` / `node.failed`.
- NEW `crates/argo-agent-runner/src/instrumentation/ir_trace.rs` — runtime hook: when a worker executes a node, emits trace event with skill_id + node_id + tenant.
- NEW `web/components/skycot/canvas/LiveTraceOverlay.tsx` — overlay component.
- NEW `web/lib/hooks/use-skill-trace.ts` — WS subscribe hook.

## Files you must NOT touch

- L1/L2 schemas — read-only.
- L4 SkillCanvas.tsx — keep core renderer decoupled; overlay is composed.
- Existing WS infrastructure (`argo-cli/src/handlers/gateway/ws_*`) — extend, don't refactor.

## Acceptance criteria

- [ ] Synthetic run emits trace events through to UI in <500ms.
- [ ] Disconnect/reconnect handled cleanly.
- [ ] Tenant-scoped (worker A's run does not leak to worker B's view).
- [ ] 4 integration tests pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `pnpm typecheck` clean in `web/`.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L5-skill-canvas-live-trace`
- PR title: `feat(web/canvas): live-trace overlay via WebSocket (L5)`

## Notes for the implementer

- Tie cleanly to replay sandbox (E5-d) — replay IS canvas-trace playback per L0.
- Use existing `argo-cli/src/handlers/gateway/ws_*` patterns (E1-f WebSocket infra).
- See `streams/L0-skill-canvas-track-overview.md` for full track context.
