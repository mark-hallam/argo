import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";
import { PublicPersonaCard } from "@/components/skycot/PublicPersonaCard";
import {
  getSamplePersona,
  SAMPLE_PERSONA_IDS,
} from "@/lib/sample-personas";
import "./page.css";

/**
 * Public marketing roster — `/teammates`. Exempt from auth middleware.
 *
 * The /landing top-nav "Roster" link used to point at /team, which
 * resolved under the (app) route group and rendered the in-app shell
 * (sidebar, demo banner, empty-state CTA) for logged-out visitors —
 * a brittle marketing-to-product handoff (e2e-ux-audit F6 [P1]).
 *
 * This page replaces that link with a public-friendly roster that
 * showcases the five canonical Day-1 teammates without exposing the
 * in-app chrome. Static, server-rendered, no session required.
 */

const PERSONAS = SAMPLE_PERSONA_IDS.map((id) => getSamplePersona(id));

export default function TeammatesPage() {
  return (
    <div className="tmm">
      <MarketingNav current="/teammates" />

      <div className="tmm-stage">
        <section className="tmm-hero">
          <div className="tmm-hero-eyebrow">
            <span className="tmm-hero-eyebrow-dot" aria-hidden />
            DAY-1 ROSTER
          </div>
          <h1 className="tmm-hero-h1">
            Meet the team you&rsquo;ll <strong>hire</strong>
          </h1>
          <p className="tmm-hero-sub">
            Five teammates the SKYCOT platform ships with on day one.
            Real personas with real responsibilities — see who&rsquo;s
            already on the team.
          </p>
        </section>

        <section className="tmm-grid-section" aria-labelledby="tmm-grid-h">
          <h2 className="tmm-grid-h" id="tmm-grid-h">
            The starting line-up
          </h2>
          <div className="tmm-grid">
            {PERSONAS.map((persona) => (
              <PublicPersonaCard key={persona.id} persona={persona} />
            ))}
          </div>
        </section>

        <section className="tmm-cta">
          <h2 className="tmm-cta-h">Build your own team</h2>
          <p className="tmm-cta-sub">
            Spin up a free demo workspace. Hire one of these five, or
            describe the role you&rsquo;d hire and the platform builds
            the teammate for you.
          </p>
          <Link href="/auth/signup" className="tmm-cta-pill" data-testid="teammates-cta">
            Try a demo workspace
            <ArrowRight size={18} aria-hidden />
          </Link>
        </section>
      </div>

      <MarketingFooter />
    </div>
  );
}
