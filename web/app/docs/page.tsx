import "./page.css";

import Link from "next/link";
import {
  ArrowRight,
  Rocket,
  GitBranch,
  Sparkles,
  Shield,
  Code2,
  Plug,
  BookOpen,
  Lightbulb,
} from "lucide-react";
import type { ReactNode } from "react";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";

/**
 * Public docs hub — `/docs`, exempt from auth middleware.
 *
 * Brand-loud editorial register matching `/landing`, `/compare`, and
 * `/changelog`: Audiowide hero, JetBrains Mono eyebrow pill, purple →
 * pink gradient emphasis on hero strong word, warm radial-blob backdrop.
 *
 * Layout: hero + 2x3 category-card grid + featured-guides band + help
 * footer with support mailto.
 *
 * Cards link to `/docs/<slug>` pseudo-routes — those will 404 gracefully
 * via the existing `not-found.tsx`. CMS-backed doc content arrives in a
 * follow-up brief.
 */

interface DocCategory {
  slug: string;
  icon: ReactNode;
  title: string;
  blurb: string;
  count: string;
}

const CATEGORIES: DocCategory[] = [
  {
    slug: "getting-started",
    icon: <Rocket size={26} />,
    title: "Getting Started",
    blurb: "Hire your first AI teammate in four minutes. Onboarding tour, your first workflow, and the trust ratchet explained.",
    count: "8 guides",
  },
  {
    slug: "workflows",
    icon: <GitBranch size={26} />,
    title: "Workflows",
    blurb: "Build, edit, and version your AI workflows. Visual canvas, plain-English editing, multi-branch logic, and approval gates.",
    count: "14 guides",
  },
  {
    slug: "skills",
    icon: <Sparkles size={26} />,
    title: "Skills",
    blurb: "Add new skills to your teammates. The skills marketplace, authoring custom skills, and tenant-scoped skill libraries.",
    count: "11 guides",
  },
  {
    slug: "audit",
    icon: <Shield size={26} />,
    title: "Audit & Compliance",
    blurb: "Forensic-grade audit chain on every action. Per-action role permissions, workspace isolation, SOC 2-ready evidence, kill-switch ops.",
    count: "9 guides",
  },
  {
    slug: "api",
    icon: <Code2 size={26} />,
    title: "API Reference",
    blurb: "REST (Representational State Transfer — a way for programs to talk to each other over the web) plus WebSocket APIs for programmatic access.",
    count: "REST + WS",
  },
  {
    slug: "integrations",
    icon: <Plug size={26} />,
    title: "Integrations",
    blurb: "Slack, Zendesk, Salesforce, Workday, HubSpot, Linear, GitHub, Stripe, and 18+ more connectors with approval-aware wiring.",
    count: "26 connectors",
  },
];

interface FeaturedGuide {
  slug: string;
  title: string;
  read: string;
  icon: ReactNode;
}

const FEATURED: FeaturedGuide[] = [
  {
    slug: "getting-started/hire-first-teammate",
    title: "Hire your first AI teammate in 4 minutes",
    read: "4 min read",
    icon: <Rocket size={18} />,
  },
  {
    slug: "workflows/visual-canvas",
    title: "The workflow builder, end to end",
    read: "9 min read",
    icon: <GitBranch size={18} />,
  },
  {
    slug: "audit/trust-ratchet",
    title: "How the trust ratchet promotes workers",
    read: "6 min read",
    icon: <Shield size={18} />,
  },
  {
    slug: "integrations/slack-approvals",
    title: "Setting up Slack approval gates",
    read: "5 min read",
    icon: <Plug size={18} />,
  },
];

export default function DocsHubPage() {
  return (
    <>
      <div className="dh">
        <MarketingNav />

        <section className="dh-hero">
          <div className="dh-hero-eyebrow">
            <span className="dh-hero-eyebrow-dot" />
            Documentation
          </div>
          <h1 className="dh-hero-h1">
            Everything you need to <strong>ship</strong>.
          </h1>
          <p className="dh-hero-sub">
            Operator guides, API (Application Programming Interface — the contract programs use to
            talk to SKYCOT) reference, workflow recipes, and integration docs.
          </p>
          <div className="dh-hero-search" role="search">
            <div className="dh-hero-search-icon">
              <BookOpen size={16} />
            </div>
            <input
              type="search"
              placeholder="Search the docs — try 'hire first teammate' or 'audit chain'"
              aria-label="Search documentation"
            />
            <span className="dh-hero-search-kbd">⌘K</span>
          </div>
        </section>

        <section className="dh-stage">
          <div className="dh-grid">
            {CATEGORIES.map((c) => (
              <Link key={c.slug} href={`/docs/${c.slug}`} className="dh-card">
                <div className="dh-card-icon">{c.icon}</div>
                <h3 className="dh-card-title">{c.title}</h3>
                <p className="dh-card-blurb">{c.blurb}</p>
                <div className="dh-card-foot">
                  <span className="dh-card-count">{c.count}</span>
                  <span className="dh-card-arrow">
                    Browse <ArrowRight size={14} />
                  </span>
                </div>
              </Link>
            ))}
          </div>

          <section className="dh-featured-section">
            <p className="dh-section-label">FEATURED GUIDES</p>
            <h2 className="dh-section-title">
              Start with the <strong>most-read</strong> ones.
            </h2>
            <div className="dh-featured-grid">
              {FEATURED.map((g) => (
                <Link key={g.slug} href={`/docs/${g.slug}`} className="dh-featured-card">
                  <div className="dh-featured-icon">{g.icon}</div>
                  <div>
                    <div className="dh-featured-title">{g.title}</div>
                    <div className="dh-featured-read">{g.read}</div>
                  </div>
                </Link>
              ))}
            </div>
          </section>
        </section>

        <section className="dh-help-band">
          <h2>Can&apos;t find what you need?</h2>
          <p>
            Our team replies within one business day. We&apos;re a small Melbourne crew building
            this in the open — talk to a human, not a bot.
          </p>
          <div className="dh-help-actions">
            <a href="mailto:support@skycot.com" className="dh-help-cta">
              <Lightbulb size={15} />
              Email support
            </a>
            <Link href="/changelog" className="dh-help-cta secondary">
              See what we shipped <ArrowRight size={14} />
            </Link>
          </div>
        </section>

        <MarketingFooter />
      </div>
    </>
  );
}
