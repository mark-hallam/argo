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
