/**
 * Demo-mode flag.
 *
 * `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` switches the SPA back to the seeded
 * mock data path ("Sam Harlow · Acmeco · Owner", six fake teammates,
 * fifteen audit rows) for the v1.0 recorded demo. In every other
 * environment — including production deployments and authenticated
 * buyer trials — the SPA reads real session identity from NextAuth and
 * real workspace data from the gateway, falling back to brand-voiced
 * empty states on fresh workspaces.
 *
 * `NEXT_PUBLIC_*` env vars are inlined by Next at build time, so this
 * boolean compiles to a static constant in the client bundle (no
 * runtime fetch). The DemoBanner in AppShell and the mock fallbacks in
 * `lib/api/{audit,team,cohorts,approvals}.ts` ride off this same flag.
 */
export const DEMO_MODE_ENV_VAR = "NEXT_PUBLIC_SKYCOT_DEMO_MODE" as const;

export function isDemoMode(): boolean {
  return process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE === "1";
}

/**
 * Recording-mode flag.
 *
 * `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` opts the recording-day SPA out of
 * the auth-bypass mock short-circuits in `lib/api/{audit,audit-detail,
 * replay,approvals}.ts`, so the audit feed, drill-in, replay, and
 * approval-decide buttons all fire real `GET /api/v1/audit*` and
 * `POST /api/v1/approvals/<id>/decide` calls against the seeded
 * gateway. The seeded persona names + avatars stay (because they live
 * in the gateway's seed data, not in the SPA's mock layer), but the
 * "Real provenance" narration in Scene 6 of the v1.0 demo MP4 is now
 * literally true — the camera sees a real backend round-trip, not a
 * `mockPage()` fabrication.
 *
 * Mode triplet:
 *  - `auth-bypass + demo-mode` (today) → mock-only, fast local UI
 *    iteration; no gateway calls.
 *  - `auth-bypass + demo-mode + demo-live` (recording day) → real
 *    gateway calls, seeded responses. Operator narration matches.
 *  - signed-in + no demo flags → production, real user data.
 *
 * Demo-live presumes `argo demo seed --tenant <demo>` has been run
 * against the live gateway so the seeded persona rows actually exist
 * server-side; without that step the audit feed renders empty.
 */
export const DEMO_LIVE_ENV_VAR = "NEXT_PUBLIC_SKYCOT_DEMO_LIVE" as const;

export function isDemoLive(): boolean {
  return process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE === "1";
}
