"use client";
// Pricing claim register: docs/skycot/marketing/PRICING.md.
import "./page.css";

import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { Suspense, useState } from "react";
import { ArrowRight, Check, ChevronDown } from "lucide-react";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";
import { EnterpriseInquiryForm } from "@/components/marketing/EnterpriseInquiryForm";
import { useSkycotSession } from "@/lib/skycot/use-session";
import {
  useOnboardingDraft,
  type SelectedPlan,
} from "@/lib/onboarding/use-onboarding-draft";
import { useCheckout, redirectToCheckout } from "@/lib/billing/use-checkout";
import { TIERS, MATRIX, FAQ, type Tier } from "@/lib/pricing/pricing-content";

/**
 * Public pricing page — `/pricing`, exempt from auth middleware.
 *
 * Brand-loud-marketing register matching `/landing` and `/customers`.
 * Three tiers (Free / Pro / Enterprise) state the Option D model (DEC-053):
 * Free = on-platform action quota; Pro = a flat fee per named teammate with
 * an included allowance, then metered overage capped by a self-set spend limit;
 * Enterprise = bring-your-own keys + flat platform fee + SSO/SLA. Tier
 * numbers, the feature matrix, and the FAQ all DERIVE from the canonical
 * `docs/skycot/pricing/v1-tiers.toml` via `@/lib/pricing/*` — there are no
 * hardcoded prices, allowances, or multipliers in this JSX (DEC-035).
 *
 * Per-tier CTAs. Signed-out → /auth/signup?plan=<tier> (you must have a
 * workspace before you can pay). Signed-in Pro → invokes `useCheckout()`
 * directly (→ POST /api/v1/billing/checkout → redirect to the Stripe hosted
 * URL); if Stripe isn't provisioned yet the CTA surfaces a truthful error
 * rather than a fake success (NO THEATER). The selected plan is persisted on
 * the onboarding draft store so the post-signup welcome step can hop to
 * checkout. `?featured=pro` campaign param visually expands the Pro card.
 */

function AvailabilityMark({ included }: { included: boolean }) {
  if (included) {
    return (
      <span className="pri-check" role="img" aria-label="Included">
        <Check size={18} aria-hidden="true" />
      </span>
    );
  }

  return (
    <span className="pri-empty" aria-label="Not included">
      -
    </span>
  );
}

export default function PricingPage() {
  return (
    <Suspense fallback={null}>
      <PricingPageContent />
    </Suspense>
  );
}

function PricingPageContent() {
  const [open, setOpen] = useState<number | null>(0);
  const session = useSkycotSession();
  const setSelectedPlan = useOnboardingDraft((s) => s.setSelectedPlan);
  const params = useSearchParams();
  const featuredOverride = params?.get("featured");
  const isSignedIn = session.status === "authenticated";
  const checkout = useCheckout();
  const [checkoutError, setCheckoutError] = useState<string | null>(null);

  function persistPlan(tier: Tier): void {
    // Only the two paid-flow tiers persist; "enterprise" is talk-to-sales.
    if (tier === "free" || tier === "pro") {
      setSelectedPlan(tier satisfies SelectedPlan);
    }
  }

  // Signed-in Pro CTA: start a real Stripe Checkout session. The price id
  // comes from NEXT_PUBLIC_STRIPE_PRICE_PRO (set once WS4-2 provisions
  // Stripe). Until then — and on any backend error — we surface a clean,
  // truthful message instead of a silent no-op or a fake success.
  async function startProCheckout(): Promise<void> {
    persistPlan("pro");
    setCheckoutError(null);
    const priceId = process.env.NEXT_PUBLIC_STRIPE_PRICE_PRO;
    if (!priceId) {
      setCheckoutError(
        "Checkout isn’t available yet — Stripe billing is still being set up. Email hello@skycot.com and we’ll get you onto Pro.",
      );
      return;
    }
    try {
      await redirectToCheckout(checkout.mutateAsync, { priceId });
    } catch (err) {
      setCheckoutError(
        err instanceof Error
          ? err.message
          : "Couldn’t start checkout. Please try again.",
      );
    }
  }

  return (
    <>
      <div className="pri">
        <MarketingNav />

        <section className="pri-hero">
          <div className="pri-eyebrow">
            <span className="pri-eyebrow-dot" />
            PRICING
          </div>
          <h1 className="pri-h1">
            Hire teammates. Pay like they&apos;re <strong>real</strong>.
          </h1>
          <p className="pri-sub">
            Start free. Pro is per named teammate with an action allowance, then
            metered overage that stops at a cap you set — no surprise bills.
            Enterprise brings your own keys.
          </p>
        </section>

        <div className="pri-tier-grid">
          {TIERS.map((t) => {
            const isFeatured = featuredOverride
              ? featuredOverride === t.tier
              : Boolean(t.featured);
            const href = isSignedIn ? t.signedInHref : t.signedOutHref;
            return (
              <div
                key={t.tier}
                className={`pri-tier${isFeatured ? " featured" : ""}`}
                data-tier={t.tier}
              >
                {isFeatured && <div className="pri-tier-ribbon">Most popular</div>}
                <div>
                  <h3 className="pri-tier-title">{t.title}</h3>
                </div>
                <div>
                  <div className={`pri-tier-price${isFeatured ? " gradient" : ""}`}>{t.price}</div>
                  <div className="pri-tier-period">{t.period}</div>
                </div>
                <p className="pri-tier-blurb">{t.blurb}</p>
                <ul className="pri-tier-features">
                  {t.features.map((f) => (
                    <li key={f}>
                      <Check size={16} />
                      <span>{f}</span>
                    </li>
                  ))}
                </ul>
                {t.tier === "pro" && isSignedIn ? (
                  <>
                    <button
                      type="button"
                      className={`pri-tier-cta ${isFeatured ? "primary" : "secondary"}`}
                      onClick={() => void startProCheckout()}
                      disabled={checkout.isPending}
                      data-testid={`pricing-cta-${t.tier}`}
                    >
                      {checkout.isPending ? (
                        "Starting checkout…"
                      ) : (
                        <>
                          {t.cta} <ArrowRight size={15} />
                        </>
                      )}
                    </button>
                    {checkoutError && (
                      <p
                        role="alert"
                        data-testid="pricing-checkout-error"
                        style={{
                          marginTop: 10,
                          fontSize: 13,
                          lineHeight: 1.45,
                          color: "var(--destructive)",
                        }}
                      >
                        {checkoutError}
                      </p>
                    )}
                  </>
                ) : (
                  <Link
                    href={href}
                    className={`pri-tier-cta ${isFeatured ? "primary" : "secondary"}`}
                    onClick={() => persistPlan(t.tier)}
                    data-testid={`pricing-cta-${t.tier}`}
                  >
                    {t.cta} <ArrowRight size={15} />
                  </Link>
                )}
              </div>
            );
          })}
        </div>

        <section className="pri-section" id="enterprise-inquiry">
          <p className="pri-section-label">ENTERPRISE</p>
          <h2 className="pri-section-title">Talk to sales.</h2>
          <p
            style={{
              maxWidth: 560,
              margin: "0 auto 28px",
              textAlign: "center",
              fontSize: 16,
              lineHeight: 1.55,
              color: "var(--muted-foreground)",
            }}
          >
            SSO, audit export, custom verticals, and a negotiated rate. Tell us
            a little about your team and we’ll be in touch within one business
            day.
          </p>
          <div style={{ maxWidth: 560, margin: "0 auto" }}>
            <EnterpriseInquiryForm source="pricing" />
          </div>
        </section>

        <section className="pri-section">
          <p className="pri-section-label">FEATURE MATRIX</p>
          <h2 className="pri-section-title">Compare every line.</h2>
          <div className="pri-matrix-wrap">
            <table className="pri-matrix">
              <caption className="sr-only">
                Pricing feature availability across Free, Pro, and Enterprise plans.
              </caption>
              <thead>
                <tr>
                  <th scope="col">Feature</th>
                  <th scope="col">Free</th>
                  <th scope="col">Pro</th>
                  <th scope="col">Enterprise</th>
                </tr>
              </thead>
              <tbody>
                {MATRIX.map((row) => (
                  <tr key={row.label}>
                    <th scope="row">{row.label}</th>
                    <td><AvailabilityMark included={row.free} /></td>
                    <td><AvailabilityMark included={row.pro} /></td>
                    <td><AvailabilityMark included={row.enterprise} /></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        <section className="pri-section">
          <p className="pri-section-label">FAQ</p>
          <h2 className="pri-section-title">Frequently asked questions.</h2>
          <div className="pri-faq">
            {FAQ.map((item, idx) => {
              const answerId = `pricing-faq-answer-${idx}`;
              return (
                <div key={item.q} className={`pri-faq-item${open === idx ? " open" : ""}`}>
                  <button
                    type="button"
                    className="pri-faq-q"
                    onClick={() => setOpen(open === idx ? null : idx)}
                    aria-expanded={open === idx}
                    aria-controls={answerId}
                  >
                    <span>{item.q}</span>
                    <ChevronDown size={18} />
                  </button>
                  <div id={answerId} className="pri-faq-a" hidden={open !== idx}>
                    {item.a}
                  </div>
                </div>
              );
            })}
          </div>
        </section>

        <section className="pri-final">
          <h2>Ready to hire your first teammate?</h2>
          <p>No credit card. No demo call. A guided interview so your first teammate starts right.</p>
          <Link href="/auth/signup" className="pri-final-cta">
            Start free <ArrowRight size={16} />
          </Link>
        </section>

        <MarketingFooter />
      </div>
    </>
  );
}
