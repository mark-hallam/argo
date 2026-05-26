"use client";

import {
  ArrowRight,
  Github,
  ShieldCheck,
  Sparkles,
  Twitter,
  Workflow,
} from "lucide-react";
import { ApprovalCard } from "@/components/ui/ApprovalCard";
import { AuditTimeline } from "@/components/ui/AuditTimeline";
import { Button } from "@/components/ui/Button";
import { ChannelBadge } from "@/components/ui/ChannelBadge";
import { CodeBlock } from "@/components/ui/CodeBlock";
import { Diagram } from "@/components/ui/Diagram";
import { FeatureSection } from "@/components/ui/FeatureSection";
import { Footer } from "@/components/ui/Footer";
import { KeyFact } from "@/components/ui/KeyFact";
import { LandingHero } from "@/components/ui/LandingHero";
import { Nav } from "@/components/ui/Nav";
import { OnboardingStep } from "@/components/ui/OnboardingStep";
import { PricingCard } from "@/components/ui/PricingCard";
import { SignupForm } from "@/components/ui/SignupForm";
import { TestimonialCard } from "@/components/ui/TestimonialCard";
import { WorkerCard } from "@/components/ui/WorkerCard";

export default function BrandComponentsPage() {
  return (
    <main className="mx-auto max-w-5xl px-6 py-12 space-y-16">
      <header className="space-y-3">
        <p className="text-sm font-mono text-[var(--muted-foreground)]">/brand/components</p>
        <h1 className="text-4xl font-bold tracking-tight text-[var(--foreground)]">
          Design-system components
        </h1>
        <p className="text-[var(--muted-foreground)] max-w-2xl">
          Marketing + onboarding + product primitives. All consume brand tokens
          from <code className="font-mono">web/app/globals.css</code>. See{" "}
          <code className="font-mono">docs/skycot/brand/DESIGN_TOKENS.md</code>.
        </p>
      </header>

      <Section title="LandingHero">
        <LandingHero
          eyebrow="Skycot"
          headline="Hire an AI team that gets work done"
          subcopy="Operational AI built for ambitious teams. Audited, observable, repeatable."
          primaryCta={<Button size="lg">Get started <ArrowRight aria-hidden /></Button>}
          secondaryCta={<Button size="lg" variant="secondary">Talk to us</Button>}
        />
      </Section>

      <Section title="SignupForm">
        <div className="grid gap-6 sm:grid-cols-3">
          <Tile label="idle">
            <SignupForm onSubmit={() => {}} />
          </Tile>
          <Tile label="loading">
            <SignupForm onSubmit={() => {}} state="loading" />
          </Tile>
          <Tile label="success">
            <SignupForm onSubmit={() => {}} state="success" />
          </Tile>
        </div>
      </Section>

      <Section title="OnboardingStep">
        <OnboardingStep
          totalSteps={4}
          currentStep={2}
          stepLabels={["Account", "Workspace", "Connectors", "Invite"]}
          headline="Name your workspace"
          body="Workspaces hold your AI workers, audit logs, and connectors."
          actions={
            <>
              <Button variant="ghost">Back</Button>
              <Button>Continue</Button>
            </>
          }
        />
      </Section>

      <Section title="PricingCard">
        <div className="grid gap-6 md:grid-cols-3">
          <PricingCard
            planName="Starter"
            price="$49"
            cadence="/mo"
            description="Solo founders and prototypes."
            features={["1 workspace", "100 actions/mo", "Email support"]}
            cta={<Button variant="secondary" className="w-full">Start free</Button>}
          />
          <PricingCard
            planName="Team"
            price="$299"
            cadence="/mo"
            description="Operating teams running production AI."
            features={["5 workspaces", "10K actions/mo", "Chat support", "Audit retention 90d"]}
            cta={<Button className="w-full">Start trial</Button>}
            featured
          />
          <PricingCard
            planName="Enterprise"
            price="Custom"
            description="Multi-tenant deploys, SSO, on-call."
            features={["SLA", "SSO + SCIM", "Audit retention 1y", "Dedicated rep"]}
            cta={<Button variant="secondary" className="w-full">Contact sales</Button>}
          />
        </div>
      </Section>

      <Section title="FeatureSection">
        <FeatureSection
          icon={<ShieldCheck className="size-5" aria-hidden />}
          eyebrow="Security"
          headline="Every action is audited"
          body="From human approval to model output, each step is captured to a tamper-evident log."
          media={
            <div className="aspect-video rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]" />
          }
        />
        <FeatureSection
          icon={<Workflow className="size-5" aria-hidden />}
          eyebrow="Workflows"
          headline="Compose without code"
          body="Drag steps, gate them with approvals, ship to production."
          media={
            <div className="aspect-video rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]" />
          }
          reverse
        />
      </Section>

      <Section title="TestimonialCard">
        <div className="grid gap-6 md:grid-cols-2">
          <TestimonialCard
            quote="Skycot collapsed our ops stack from four tools to one. The audit trail alone justified it."
            name="Alex Rivera"
            role="Head of Operations"
            company="Northwind"
          />
          <TestimonialCard
            quote="We shipped an AI receptionist in a weekend. Two weeks later it had handled 1,200 calls."
            name="Priya Kapoor"
            role="Founder"
            company="Tilden"
          />
        </div>
      </Section>

      <Section title="KeyFact">
        <div className="grid gap-6 sm:grid-cols-3">
          <KeyFact value="92%" label="Approval rate" context="Across all Skycot tenants, last 30 days." />
          <KeyFact value="3.1s" label="Median response" context="Tool call → human-readable reply." />
          <KeyFact value="1.2K" label="Audits/day" context="Per production tenant." align="center" />
        </div>
      </Section>

      <Section title="CodeBlock">
        <CodeBlock
          language="bash"
          code={"# install skycot\ncurl -sSf https://skycot.com/install | sh\nargo --version"}
        />
      </Section>

      <Section title="Diagram">
        <Diagram ariaLabel="Skycot core platform layers" caption="Skycot architecture (illustrative).">
          <svg viewBox="0 0 320 120" xmlns="http://www.w3.org/2000/svg" role="presentation">
            <rect x="10" y="10" width="300" height="30" rx="6" className="fill-[var(--muted)]" />
            <text x="160" y="30" textAnchor="middle" className="fill-[var(--foreground)] text-[10px]">
              Vertical agent templates
            </text>
            <rect x="10" y="50" width="300" height="30" rx="6" className="fill-[var(--card)]" />
            <text x="160" y="70" textAnchor="middle" className="fill-[var(--foreground)] text-[10px]">
              Core platform — runtime / RBAC / audit
            </text>
            <rect x="10" y="90" width="300" height="20" rx="6" className="fill-[var(--brand-primary)]/30" />
            <text x="160" y="104" textAnchor="middle" className="fill-[var(--foreground)] text-[10px]">
              Providers + MCP connectors
            </text>
          </svg>
        </Diagram>
      </Section>

      <Section title="Footer">
        <Footer
          brand={<span className="font-bold tracking-tight">skycot</span>}
          tagline="Operational AI for ambitious teams."
          sections={[
            { title: "Product", links: [{ label: "Features", href: "/features" }, { label: "Pricing", href: "/pricing" }] },
            { title: "Docs", links: [{ label: "Quickstart", href: "/docs/quickstart" }, { label: "API", href: "/docs/api" }] },
            { title: "Company", links: [{ label: "About", href: "/about" }, { label: "Careers", href: "/careers" }] },
            { title: "Legal", links: [{ label: "Privacy", href: "/legal/privacy" }, { label: "Terms", href: "/legal/terms" }] },
          ]}
          social={[
            { label: "GitHub", href: "https://github.com/", icon: Github },
            { label: "Twitter", href: "https://twitter.com/", icon: Twitter },
          ]}
          copyright="© 2026 Skycot, Inc."
        />
      </Section>

      <Section title="Nav">
        <div className="rounded-[var(--radius-md)] border border-[var(--border)] overflow-hidden">
          <Nav
            brand={<span className="font-bold tracking-tight">skycot</span>}
            links={[
              { label: "Features", href: "/features" },
              { label: "Pricing", href: "/pricing" },
              { label: "Docs", href: "/docs" },
            ]}
            cta={<Button size="sm">Sign in</Button>}
            sticky={false}
          />
        </div>
      </Section>

      <Section title="ChannelBadge">
        <div className="flex flex-wrap gap-2">
          <ChannelBadge kind="slack" />
          <ChannelBadge kind="email" />
          <ChannelBadge kind="sms" />
          <ChannelBadge kind="voice" />
        </div>
      </Section>

      <Section title="AuditTimeline">
        <AuditTimeline
          rows={[
            { id: "1", timestamp: "2026-05-16T10:00:00Z", kind: "approval.requested", tone: "info", payload: "refund $129 to user 42" },
            { id: "2", timestamp: "2026-05-16T10:01:14Z", kind: "approval.granted", tone: "success", payload: "by alice@" },
            { id: "3", timestamp: "2026-05-16T10:01:42Z", kind: "tool.refund.success", tone: "success" },
          ]}
        />
      </Section>

      <Section title="WorkerCard">
        <div className="grid gap-6 md:grid-cols-2">
          <WorkerCard
            name="Receptionist"
            role="Voice + email triage"
            status="working"
            duties={["Greet incoming calls", "Route to humans on escalation"]}
          />
          <WorkerCard
            name="Support copilot"
            role="Tier-1 ticket draft"
            status="waiting"
            duties={["Drafts replies in Zendesk", "Pauses on approval"]}
          />
        </div>
      </Section>

      <Section title="ApprovalCard">
        <div className="grid gap-6 md:grid-cols-2">
          <ApprovalCard
            title="Refund $129 to user 42"
            summary="Initiated by receptionist agent in tenant northwind."
            requestedBy={<><Sparkles aria-hidden className="inline size-3 mr-1" />receptionist</>}
            requestedAt="2026-05-16T10:00:00Z"
            payload={`{
  "amount_cents": 12900,
  "card_last4": "4242"
}`}
          />
          <ApprovalCard
            title="Send broadcast to 1.2K customers"
            summary="Marketing agent drafted; awaiting human approval."
            state="approved"
            requestedAt="2026-05-16T09:30:00Z"
          />
        </div>
      </Section>
    </main>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="space-y-4">
      <h2 className="text-xl font-semibold tracking-tight text-[var(--foreground)] border-b border-[var(--border)] pb-2">
        {title}
      </h2>
      {children}
    </section>
  );
}

function Tile({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-2">
      <p className="text-xs font-mono text-[var(--muted-foreground)]">{label}</p>
      {children}
    </div>
  );
}
