import { redirect } from "next/navigation";

/**
 * `/onboarding` index — bare-URL entry point that lands users on the
 * welcome step. Without this server component, `/onboarding` 404s
 * (no `page.tsx` at the route-group root), even though
 * `/onboarding/welcome` and the 4 subsequent step routes have shipped
 * since the F1 design lock on 2026-05-19. The 404 silently breaks
 * `/auth/verify`'s `router.replace("/onboarding/welcome")` for any
 * inbound link that drops the `/welcome` suffix (magic-link emails,
 * shared URLs, the `safeReturnToOrHome` allowlist which permits the
 * `/onboarding` prefix exactly).
 */
export default function OnboardingIndexPage(): never {
  redirect("/onboarding/welcome");
}
