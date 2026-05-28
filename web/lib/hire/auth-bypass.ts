/**
 * Auth-bypass detection for the hire wizard.
 *
 * The wizard requires a signed-in session in any production-style
 * deployment, but several non-production modes deliberately accept
 * anonymous POSTs against the gateway:
 *
 *   - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` — local dev + recording
 *     mode; gateway accepts unauthenticated `POST /api/v1/workers`.
 *   - `NEXT_PUBLIC_ARGO_DEMO_MODE=1` — the demo-mint signup variant
 *     used by the v1.0 recording session.
 *   - `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` — the seeded "Sam Harlow · Acmeco"
 *     identity used by mock fallbacks in `lib/api/*`.
 *
 * Returning `true` here tells the step-1/[step] guards to fall through
 * to the existing wizard flow instead of redirecting to `/auth/signup`.
 * `NEXT_PUBLIC_*` env vars are inlined at build time, so this compiles
 * to a static branch in the client bundle.
 */
export function isHireAuthBypassed(): boolean {
  if (typeof process === "undefined") return false;
  return (
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1" ||
    process.env.NEXT_PUBLIC_ARGO_DEMO_MODE === "1" ||
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE === "1"
  );
}
