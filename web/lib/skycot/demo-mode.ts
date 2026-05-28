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

/**
 * Gateway-auth-bypass flag — readable from the SPA bundle.
 *
 * `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` is the dev-only switch the
 * 28 fetchers in `web/lib/api/*.ts` consult before deciding whether to
 * short-circuit to seed fixtures. The check is also gated on
 * `NODE_ENV !== "production"`, so a production bundle ignores the env
 * var even if a hostile build sets it.
 *
 * This SPA-side mirror lives next to `isDemoMode()` / `isDemoLive()`
 * for two reasons: the recording-mode readiness warning below reads
 * all three flags together, and a single export keeps the
 * "what state is the SPA in?" decision in one module instead of
 * duplicated across every fetcher.
 */
export const GATEWAY_AUTH_BYPASS_ENV_VAR =
  "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS" as const;

export function isGatewayAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

/**
 * Recording-prep readiness warning.
 *
 * Returns a single-line warning when the SPA boots with both
 * `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` AND
 * `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. Under that combination the
 * 28 `lib/api/*` fetchers short-circuit to seed fixtures before any
 * gateway request leaves the browser — the operator cannot pre-flight
 * the live gateway against the SPA rendering, so the demo recording
 * would narrate "Skills marketplace returns 6 cards from the gateway"
 * while the Network panel shows zero gateway calls.
 *
 * Returning `null` (the no-op steady state) keeps offline UI iteration
 * silent. The DemoBanner consumes the string via a console.warn on
 * mount; we deliberately do NOT render a visible banner because the
 * normal offline-dev profile sets both flags and the operator would
 * otherwise see a permanent yellow stripe under DemoBanner during
 * routine local work.
 */
export function recordingModeReadinessWarning(): string | null {
  if (isDemoMode() && isGatewayAuthBypassed()) {
    return (
      "⚠ DEMO-RECORDING WARNING: NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 and " +
      "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 are both set — the SPA " +
      "will short-circuit to seed fixtures and will NOT hit the live " +
      "gateway. Fine for offline UI work; for demo-recording prep set " +
      "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=0 so /skills, /team, " +
      "/audit etc. exercise the real backend. See " +
      "docs/skycot/demos/v1-mission-mvp-runbook.md → Recording-mode " +
      "env profile."
    );
  }
  return null;
}
