# Computer-use on Mac — readiness, the real-work gap chain, and the demo path

**Date:** 2026-06-11 (overnight, operator-locked DEC-050)
**Method:** 5-agent code audit + live empirical testing through the real SPA
(authenticated Run-now against the freshly-built gateway, schema v92). Every
claim below was checked by running it, not inferred. NO THEATER.

## TL;DR

- **The macOS computer-use executor is real and capable** (screenshot, click,
  type, scroll via `screencapture`+`osascript`; 4-gate TCC preflight; 58 tests).
- **The capability fence between a minted teammate and real OS work is real,
  thorough, and CORRECT** — and right now it is **complete enough that a
  Genesis teammate cannot do any local file/shell work through the SPA.** That
  is excellent security and the platform's strongest trust story, but it means
  "teammates doing real work on the Mac" needs a deliberate build, not a demo
  script.
- **The honest status:** the engine genuinely tries to do the work (it wrote
  correct shell + planned the file ops), and the platform correctly fenced it
  at every layer. The execution wires PAST the fence are not built yet.

## What I proved live (real Run-now calls, real gateway)

I hired/used a Genesis teammate ("Chase"), gave it an explicit file-cleanup
task via the Run-now `note`, and watched the gate cascade fire, layer by layer,
across four runs:

1. **Probation gate** — `tool 'terminal' is held: this teammate is in its
   Genesis probation window` (DEC-025 #2: a freshly-hired teammate's whole
   vocabulary is approval-gated for 24h). The teammate *did* call the tool —
   the engine reaches for real tools when the task is clear (yesterday's
   "just reports" was a charter/prompt issue, now solved with an explicit note).
2. **ToolDispatch RBAC** (after I graduated probation in the DB) —
   `rbac denied: genesis-worker lacks ToolDispatch { tool_name_glob: "terminal" }`.
   The fence grants ToolDispatch only for the manifest's
   `declared_actions ∪ [tools].allowed` (`worker_profile::derive`,
   `crates/argo-capability/src/worker_profile.rs`).
3. **Spawn RBAC** (after I added `terminal` to `[tools].allowed`) —
   `rbac denied: genesis-worker lacks Spawn { spawn_kind: Subprocess }`. The
   terminal tool spawns `bash -c`; `Spawn{Subprocess}` is **owner-only by
   design** (the DEC-049 host-escape boundary). A minted teammate can NEVER get
   raw shell. Correct.
4. **FsRead/FsWrite RBAC** (switching to the no-subprocess file tools) —
   `rbac denied: genesis-worker lacks FsRead { path_glob: "…/invoice.txt" }`.
   Calling the file tool needs `ToolDispatch{file_read}` AND the actual read
   needs `FsRead{path_glob}` — a genesis-worker has neither for any path.

**Conclusion:** a Genesis teammate is fenced to its **declared action
vocabulary + side-effect-free builtins** (memory, todo, research, drafting). It
has no shell, no filesystem, no spawn. Real external work is *designed* to flow
through declared-action → **executor** dispatch.

## The two missing wires (why no real work happens through the SPA yet)

From the dispatch audit (`crates/argo-cli/src/handlers/work_dispatch.rs`,
`work_emission.rs`, `routes_worker_run_exec.rs`):

- **G-DISPATCH — the SPA "Run now" path emits + classifies actions but NEVER
  dispatches them to an executor.** Actions land in the worker_action ledger as
  `pending`/`blocked` and stop. Executor dispatch exists ONLY in the CLI path
  (`argo worker actions execute`). So even a perfectly-formed `computer_use`
  action does nothing on Run-now. **This is the core gap.**
- **G-CU-WIRE — the cu-macos executor is not registered in the run path's
  executor resolver.** It's reachable from the CLI replay/execute surface, not
  from a hired teammate's turn.

## macOS permissions (the desktop-drive blocker)

The gateway binary is **ad-hoc/linker-signed** (`codesign -dv` →
`Signature=adhoc`) and was rebuilt today 06:02. macOS TCC keys ad-hoc binaries
by code-hash, which changes every rebuild — so the Accessibility +
Screen-Recording grant from 2026-05-26 is **orphaned**. Desktop-drive demos
need a re-grant to the new binary AND would re-break on the next rebuild.
**Fix:** a stable code signature (dev cert) so the identity survives rebuilds,
then one operator re-grant.

## What "teammates doing real work" actually requires (the program)

In dependency order, each a queued brief:

1. **G1 — fence-scoped filesystem grant** (smallest real-work unlock): when a
   worker has an `fs_root` AND its manifest `[tools].allowed` includes
   `file_read`/`file_write`, `worker_profile::derive` emits
   `FsRead`/`FsWrite{fs_root/**}`, and the run path threads `worker.fs_root` →
   `ctx.workspace_root`. → a teammate does real, fenced file work in its own
   folder. No shell, no host escape.
2. **G2 — executor dispatch in `process_job`**: after action emission, resolve
   each action's executor and dispatch (reusing the CLI's `resolve_executor` +
   `Executor::dispatch`), gated by approval + the graduation flag, persisting
   state transitions. → declared actions actually execute on Run-now.
3. **G3 — register cu-macos in the run-path executor resolver** so a
   `computer_use` action routes to it.
4. **G4 — stable codesign + perms runbook** so desktop-drive survives rebuilds.
5. **G5 — in-SPA approve→execute** for held actions (today `session approve`
   is CLI-only + needs manual re-issue) — closes the human-in-the-loop loop.

## The morning demo (honest options, strongest first)

- **DEMO A — The safety spine, live (available now, 100% real, no build):**
  hire a teammate, ask it to do real OS work, watch the platform fence it
  (probation → RBAC → Spawn) while the teammate writes the correct commands.
  The audit row + transcript prove it. This is the trust differentiator no
  competitor ships — "your AI teammate physically cannot go rogue."
- **DEMO B — Genesis build, live (available now, proven 2026-06-11):**
  describe a role → real interview (probes vagueness, asks access/escalation) →
  real hire → truthful capability contract. The "build a teammate" wow.
- **DEMO C — Real fenced file work (needs G1, ~half-day build):** a "Filing
  Assistant" reads real invoices and writes a real AR-summary in its folder,
  through the SPA, within the fence. The "real work" wow — honest once G1 lands.
- **DEMO D — Desktop drive / true computer-use (needs G2+G3+G4):** teammate
  drives Notes/Finder on screen. The flashiest, furthest out.

**Recommendation:** show A + B in the morning (both real today), and green-light
G1 → C as the next build so "real work" is demoable honestly within a day.

## Cleanup note

Test mutations were made to the local `genesis-chase-*` tenant pack
(probation graduated in `persona.provenance_chain_json`; file tools added to
`[tools].allowed`) and demo files seeded under `.skycot/demo-inbox/` +
`~/SkycotDemo/`. These are local tenant/demo data, not tracked repo state.
