"use client";
// Pricing claim register: docs/skycot/marketing/PRICING.md.
import "./page.css";

import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { Suspense, useState } from "react";
import { ArrowRight, Check, ChevronDown } from "lucide-react";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";
import { useSkycotSession } from "@/lib/skycot/use-session";
import {
  useOnboardingDraft,
  type SelectedPlan,
} from "@/lib/onboarding/use-onboarding-draft";

/**
 * Public pricing page — `/pricing`, exempt from auth middleware.
 *
 * Brand-loud-marketing register matching `/landing` and `/customers`.
 * Three tiers (Free / Pro / Enterprise) mirror the `/billing` PLANS
 * constant in `web/app/(app)/billing/page.tsx`. Feature comparison
 * matrix + FAQ + final CTA.
 *
 * Per-tier deep-link CTAs (signed-out → /auth/signup?plan=<tier>, signed-in
 * Pro → /billing/subscribe?plan=pro). The selected plan is persisted on the
 * onboarding draft store so the post-signup welcome step can hop to
 * checkout. `?featured=pro` campaign param visually expands the Pro card.
 *
 * V1 ships static copy; CMS-driven feature matrix arrives in a
 * follow-up brief.
 */

type Tier = "free" | "pro" | "enterprise";

interface TierCard {
  tier: Tier;
  title: string;
  price: string;
  period: string;
  blurb: string;
  features: string[];
  cta: string;
  /** Static signed-out target; Pro overrides this when signed in. */
  signedOutHref: string;
  signedInHref: string;
  /** Default featured (visually highlighted) state. */
  featured?: boolean;
}

const TIERS: TierCard[] = [
  {
    tier: "free",
    title: "Free",
    price: "$0",
    period: "forever",
    blurb: "Try SKYCOT with the mock provider. No credit card.",
    features: [
      "1 teammate",
      "100 actions / month",
      "Community support",
      "Mock provider only",
      "Single channel (web inbox)",
    ],
    cta: "Choose Free",
    signedOutHref: "/auth/signup?plan=free",
    signedInHref: "/onboarding/welcome",
  },
  {
    tier: "pro",
    title: "Pro",
    price: "$29",
    period: "per teammate / month",
    blurb: "Real provider keys, multi-channel, marketplace skills.",
    features: [
      "5 teammates",
      "5,000 actions / month",
      "Email support",
      "Real provider keys (OpenAI / Anthropic / Bedrock)",
      "Multi-channel (Slack / iMessage / Email)",
      "Approval flows + trust ratchet",
      "Marketplace skills",
      "Versioned workflows + rollback",
    ],
    cta: "Choose Pro",
    signedOutHref: "/auth/signup?plan=pro",
    signedInHref: "/billing/subscribe?plan=pro",
    featured: true,
  },
  {
    tier: "enterprise",
    title: "Enterprise",
    price: "Custom",
    period: "talk to us",
    blurb: "SSO, audit export, custom verticals, priority queue.",
    features: [
      "Unlimited teammates",
      "Unlimited actions",
      "Dedicated support",
      "SSO / SAML",
      "Audit export (S3 / GCS / Azure)",
      "Custom verticals + private marketplace",
      "Priority queue + dedicated infra",
      "On-prem / single-binary deploy",
    ],
    cta: "Talk to sales",
    signedOutHref: "mailto:hello@skycot.com?subject=Enterprise%20pricing",
    signedInHref: "mailto:hello@skycot.com?subject=Enterprise%20pricing",
  },
];

interface MatrixRow {
  label: string;
  free: boolean;
  pro: boolean;
  enterprise: boolean;
}

const MATRIX: MatrixRow[] = [
  { label: "SOC 2-ready audit trail", free: true, pro: true, enterprise: true },
  { label: "GDPR workflow tools", free: true, pro: true, enterprise: true },
  { label: "HIPAA-aligned redaction", free: false, pro: true, enterprise: true },
  { label: "ISO 27001 controls supported", free: false, pro: true, enterprise: true },
  { label: "Hash-chain + immutable log (regulator-ready)", free: false, pro: false, enterprise: true },
  { label: "Encryption-at-rest (SQLCipher)", free: true, pro: true, enterprise: true },
  { label: "Per-action role permissions", free: true, pro: true, enterprise: true },
  { label: "Forensic audit chain", free: true, pro: true, enterprise: true },
  { label: "Sub-workflows + delegation", free: false, pro: true, enterprise: true },
  { label: "Custom verticals + prompt packs", free: false, pro: false, enterprise: true },
  { label: "BYO model keys", free: false, pro: true, enterprise: true },
  { label: "SSO / SAML", free: false, pro: false, enterprise: true },
];

interface FaqItem {
  q: string;
  a: string;
}

const FAQ: FaqItem[] = [
  {
    q: "How does per-teammate billing work?",
    a: "On Pro, every named teammate you hire (Mariah, Priya, Jamal, etc.) is $29/month. You only pay for teammates that are actively working — pause them and billing stops. Enterprise is a flat negotiated rate regardless of teammate count.",
  },
  {
    q: "Can I bring my own AI model keys?",
    a: "Yes, on Pro and Enterprise. Add your OpenAI, Anthropic, Bedrock, or self-hosted endpoint keys in Settings → Providers. SKYCOT routes through them transparently and you pay the model provider directly for inference.",
  },
  {
    q: "What happens if my teammate goes over my action limit?",
    a: "On Free, the teammate pauses and asks you to upgrade. On Pro, you get overage at $0.01 per action with an alert at 80% used. On Enterprise, limits are negotiated up-front — no surprises.",
  },
  {
    q: "Is data shared between tenants?",
    a: "No. Each customer workspace is fully isolated — every audit entry, skill run, integration call, and memory key carries the workspace stamp, and our automated checks block any code that could leak across the boundary. Tenant isolation is the load-bearing security guarantee.",
  },
  {
    q: "Can I self-host SKYCOT?",
    a: "Yes, on Enterprise. SKYCOT ships as a single binary with SQLite + SQLCipher, runs on a Mac mini or any Linux server, and includes a Docker image. The same binary you run locally runs in production — no separate self-host build.",
  },
  {
    q: "How is this different from Zapier / n8n?",
    a: "Zapier and n8n call them 'zaps' and 'workflows'. SKYCOT teammates have names, traits, and signature behaviors — they show up in Slack like a colleague, ask before they act, and earn trust over time. See /compare for the side-by-side breakdown.",
  },
];

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

  function persistPlan(tier: Tier): void {
    // Only the two paid-flow tiers persist; "enterprise" is talk-to-sales.
    if (tier === "free" || tier === "pro") {
      setSelectedPlan(tier satisfies SelectedPlan);
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
            Start free. Add a Pro tier when you&apos;re ready for production. Talk to us for enterprise.
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
                <Link
                  href={href}
                  className={`pri-tier-cta ${isFeatured ? "primary" : "secondary"}`}
                  onClick={() => persistPlan(t.tier)}
                  data-testid={`pricing-cta-${t.tier}`}
                >
                  {t.cta} <ArrowRight size={15} />
                </Link>
              </div>
            );
          })}
        </div>

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
          <p>No credit card. No demo call. Four minutes to your first working teammate.</p>
          <Link href="/auth/signup" className="pri-final-cta">
            Start free <ArrowRight size={16} />
          </Link>
        </section>

        <MarketingFooter />
      </div>
    </>
  );
}
