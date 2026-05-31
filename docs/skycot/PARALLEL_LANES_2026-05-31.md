# Parallel build lanes — 2026-05-31

Three AI sessions building SKYCOT in parallel, coordinating ONLY via git
(`origin/main`). This is the coordination map; per-session detail is in the
handoff docs linked at the bottom.

## Lanes

| Worker | Machine / clone | Lane | Verifies via |
|---|---|---|---|
| **Mac primary** (conductor) | this Mac, `~/code/argo` | Genesis (`argo-genesis`, `routes_genesis*` / `routes_workers_hire*` / `routes_onboarding*`), the hero-teammate consoles (`/genesis`, `/pitch`, `/gtm`, `/marketing`) + their web clients, the live gateway, demo | builds + **runs the live `:7400` gateway + `:3000` SPA** (the integrator) |
| **Codex** | this Mac, `argo-codex` clone (Node-only, light) | the rest of the **frontend** — `web/` in-product pages (workers, audit, skills, workflows, cohorts, approvals, settings, onboarding, team, billing, verticals) + marketing/public pages, EXCLUDING the hero consoles | `tsc` + `vitest` (live render confirmed by Mac primary) |
| **Windows** | separate Windows machine | backend **Rust library crates** — non-Genesis, NON-`argo-cli` (`argo-cli` won't compile on Windows: the kryptering/SAML LLP64 chain) | `cargo test` / `clippy` (live behavior confirmed by Mac primary) |

## Integrator = Mac primary
Pulls all three lanes' pushes into the live clone and **live-verifies the
running result** — Codex's frontend on `:3000`, Windows's backend behavior on
`:7400`. The other two lanes' "done" = green local tests; the *live*
confirmation is the Mac's.

## Coordination rules
- Git is the bus. **Rebase before EVERY push; never force-push.** Lanes touch
  different files, so merges are mostly clean.
- Shared-file contention point = `docs/build-plan/FEATURE_MATRIX.md` + its
  generated HTML (all three update it). Different rows auto-merge; re-run
  `python3 scripts/render-feature-matrix.py` on an HTML conflict. The Mac
  periodically checks the HTML matches the `.md`.
- Brand is LOCKED (`docs/skycot/brand/DESIGN_SYSTEM.md`). Truth-bounded
  reporting (DEC-029). NO THEATER (DEC-007).

## State snapshot (2026-05-31)
- **Mac primary:** Genesis P0–P7 done + deployed + live-verified on `:7400`;
  both `/genesis` browser-flow gaps fixed (SSE proxy + tenant-segment); Marcus
  (marketing teammate, DEC-028 v0) live + verified; route-swap (`676be3ffa`)
  compiles on the Mac.
- **Codex:** frontend de-theatre / locked-design ports / tsc-cleanup (its lane).
- **Windows:** Phases 0–2 done (compat map, test-health, WorkerSessionRunner
  production adapters); Phase 3 in progress.

## Per-session detail
- Windows: `docs/skycot/windows-backend-session-2026-05-31.md`
- Codex: `docs/skycot/codex-frontend-session-2026-05-31.md`
- Mac primary: this conductor session (state in `git log` + `FEATURE_MATRIX.md`).
