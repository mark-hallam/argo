import { redirect } from "next/navigation";

/**
 * `/help` — redirect to `/docs` until the in-product help center ships.
 *
 * The v1.0 placeholder this replaced was an SEO + discoverability trap
 * (whole-site-mvp-claims-coverage F08): three escape-hatch links sitting
 * under a "we're building" headline. The existing inbound links —
 * WizardEditorialShell + auth shells' top-right "Help" anchor,
 * `/sla` footer, `/auth/signin` body — all land on `/docs` via this
 * server-side 307 with no broken-link churn.
 *
 * When the real help center ships (v1.1), replace this redirect with
 * the page body and unlist `/help` from the marketing nav (already
 * pointing at `/docs`).
 */
export default function HelpPage(): never {
  redirect("/docs");
}
