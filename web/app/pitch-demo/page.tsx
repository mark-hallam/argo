import type { Metadata } from "next";
import Link from "next/link";
import { ArrowLeft, ShieldCheck, MessageSquareText, ScrollText } from "lucide-react";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";
import { PortiaEmbed } from "@/components/marketing/PortiaEmbed";
// Reuse the locked marketing brand stylesheet (`.land*` + `.portia-*` tokens).
// PortiaEmbed renders with the `portia-*` classes defined there, and the page
// shell borrows the `land-section*` editorial scale so this surface matches the
// landing page it is linked from. No new design language — the brand is locked.
import "../landing/page.css";

/**
 * `/pitch-demo` — the PUBLIC, read-only proof of the headline truth-boundary
 * differentiator ("An AI that knows what it's not allowed to say").
 *
 * Why this page exists: the landing differentiator card CTA "Watch Portia
 * refuse to overclaim" previously pointed at `/pitch`, which sits inside the
 * auth-gated `(app)/` group — so a logged-out prospect clicking it bounced
 * straight to `/auth/signin`. The truth-boundary demo was unreachable for
 * exactly the audience it exists to convince. This route is a thin, zero-
 * friction public lane over the SAME live, anonymous, per-IP-rate-limited
 * endpoint the landing-page embed already uses (`POST /api/v1/pitch/ask/public`
 * via `PortiaEmbed`): no auth, no bypass into the rest of the app — just a
 * focused, shareable surface where a stranger can watch the truth boundary work.
 *
 * It is added to the middleware public allowlist (`/pitch-demo` prefix) so it
 * never redirects to signin. The full operator rehearsal console stays at the
 * auth-gated `/pitch`.
 */

export const metadata: Metadata = {
  title: "Watch Portia refuse to overclaim — SKYCOT",
  description:
    "Portia is SKYCOT's truth-bounded Pitch Agent. Ask her a hard question and watch the claim-level truth boundary stop her stating anything she can't back up — live, no signup.",
  alternates: { canonical: "/pitch-demo" },
};

// The three beats of the truth-boundary story, shown under the live embed so a
// first-time visitor understands WHAT they just watched happen.
const HOW_IT_WORKS = [
  {
    icon: <MessageSquareText size={20} />,
    title: "She answers from a reviewed corpus",
    body: "Portia only speaks from an operator-reviewed knowledge base of SKYCOT facts. On a match she answers instantly and grounded — not from a model's imagination.",
  },
  {
    icon: <ShieldCheck size={20} />,
    title: "She defers what she can't back up",
    body: "Ask for a number she has no source for — like pricing — and the claim-level truth boundary blocks it. She defers to a human instead of guessing. That refusal is the feature.",
  },
  {
    icon: <ScrollText size={20} />,
    title: "Every answer is labelled",
    body: "Each reply carries its truth-boundary disposition — cleared or deferred — so you always see whether Portia stood behind the claim or handed it off.",
  },
];

export default function PublicPitchDemoPage() {
  return (
    <>
      <div className="land">
        <MarketingNav current="/landing#portia" />

        <section className="land-section" style={{ paddingBottom: 24 }}>
          <p className="land-section-label">LIVE · TRUTH-BOUNDED · NO SIGNUP</p>
          <h2 className="land-section-title">
            Watch Portia <strong>refuse to overclaim</strong>.
          </h2>
          <p className="land-portia-lede">
            Portia is SKYCOT&apos;s Pitch Agent. She is truth-bounded: she
            literally cannot state a number she can&apos;t back up. Ask her a
            hard question — try pricing — and watch the claim-level truth
            boundary work in real time. This is the live agent, not a recording.
          </p>
          <div className="land-portia-wrap">
            <PortiaEmbed />
          </div>
        </section>

        <section className="land-section" style={{ paddingTop: 8 }}>
          <p className="land-section-label">WHY THIS MATTERS</p>
          <h2 className="land-section-title">
            An AI that knows what it&apos;s <strong>not allowed to say</strong>.
          </h2>
          <div
            className="land-how-grid"
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))",
              gap: 16,
              maxWidth: 980,
              margin: "0 auto",
            }}
          >
            {HOW_IT_WORKS.map((step) => (
              <div
                key={step.title}
                style={{
                  border: "1px solid var(--border)",
                  borderRadius: 16,
                  padding: "22px 20px",
                  background: "var(--card)",
                  display: "flex",
                  flexDirection: "column",
                  gap: 10,
                }}
              >
                <span
                  aria-hidden="true"
                  style={{
                    display: "inline-flex",
                    width: 38,
                    height: 38,
                    borderRadius: 10,
                    alignItems: "center",
                    justifyContent: "center",
                    backgroundImage: "var(--brand-gradient)",
                    color: "var(--brand-gradient-foreground)",
                  }}
                >
                  {step.icon}
                </span>
                <h3
                  style={{
                    fontSize: 17,
                    fontWeight: 600,
                    margin: 0,
                    color: "var(--foreground)",
                  }}
                >
                  {step.title}
                </h3>
                <p
                  style={{
                    fontSize: 14.5,
                    lineHeight: 1.6,
                    margin: 0,
                    color: "var(--muted-foreground)",
                  }}
                >
                  {step.body}
                </p>
              </div>
            ))}
          </div>

          <div
            style={{
              display: "flex",
              flexWrap: "wrap",
              gap: 14,
              justifyContent: "center",
              alignItems: "center",
              marginTop: 44,
            }}
          >
            <Link
              href="/auth/signup"
              style={{
                display: "inline-block",
                padding: "12px 24px",
                backgroundImage: "var(--brand-gradient)",
                color: "var(--brand-primary-foreground)",
                textDecoration: "none",
                borderRadius: 8,
                fontWeight: 600,
                fontSize: 15,
              }}
            >
              Hire your first teammate
            </Link>
            <Link
              href="/landing#why"
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 8,
                padding: "12px 20px",
                color: "var(--foreground)",
                textDecoration: "none",
                borderRadius: 8,
                fontWeight: 600,
                fontSize: 15,
                border: "1px solid var(--border)",
              }}
            >
              <ArrowLeft size={16} /> See all four differentiators
            </Link>
          </div>
        </section>

        <MarketingFooter />
      </div>
    </>
  );
}
