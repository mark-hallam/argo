"use client";
import "./page.css";

import { use } from "react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { ArrowLeft, ArrowRight, Database, GitBranch, MessageSquare, Play, ShieldCheck, Sparkles } from "lucide-react";

import { getCustomerStory } from "@/lib/content/customers";
import { getTemplate, type TemplateStepKind } from "@/lib/workflow-templates";

/**
 * Per-customer case-study detail — `/customers/[slug]`, exempt from auth.
 *
 * Brand-loud register matching the index. 7-section layout:
 *   1. Hero strip (logo + name + industry/size + headline KPI)
 *   2. Editorial pull-quote
 *   3. The setup — 3-4 paragraph story
 *   4. Workflow showcase — embedded step list pulled from
 *      `web/lib/workflow-templates.ts` so there is no parallel
 *      workflow source of truth
 *   5. 3-tile metrics block
 *   6. Second quote from a different stakeholder
 *   7. Final CTA band
 *
 * Slug → story lookup goes through `getCustomerStory`. Unknown slug →
 * `notFound()` so Next.js renders the global `not-found.tsx`.
 */

const STEP_KIND_META: Record<TemplateStepKind, { icon: React.ReactNode; color: string; label: string }> = {
  trigger: { icon: <Play size={14} />, color: "#10b981", label: "Trigger" },
  skill_invoke: { icon: <Sparkles size={14} />, color: "#8b5cf6", label: "Skill" },
  approval: { icon: <ShieldCheck size={14} />, color: "#f59e0b", label: "Approval gate" },
  tool_call: { icon: <Database size={14} />, color: "#0891b2", label: "Tool call" },
  notify: { icon: <MessageSquare size={14} />, color: "#ec4899", label: "Notify" },
  decision: { icon: <GitBranch size={14} />, color: "#6366f1", label: "Decision" },
  data: { icon: <Database size={14} />, color: "#52525b", label: "Data" },
};

export default function CustomerStoryPage({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = use(params);
  const story = getCustomerStory(slug);
  if (!story) {
    notFound();
  }

  const template = getTemplate(story.workflowId);
  const templateHref = template ? `/workflows/templates/${template.id}` : "/workflows/templates";

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <div className="cs">
        <header className="cs-top">
          <Link href="/landing" className="cs-brand">
            <div className="cs-brand-mark" />
            <div className="cs-brand-name">SKYCOT</div>
          </Link>
          <div className="cs-top-right">
            <Link href="/auth/signin" className="cs-signin">Sign in</Link>
            <Link href="/auth/signup" className="cs-cta-pill">Hire your first teammate</Link>
          </div>
        </header>

        <div className="cs-stage">
          <Link href="/customers" className="cs-back">
            <ArrowLeft size={12} /> All customers
          </Link>

          {/* 1. Hero strip */}
          <section className="cs-hero">
            <div className="cs-hero-logo" style={{ background: story.logoGradient }}>{story.logoInitials}</div>
            <div>
              <div className="cs-hero-meta">
                <span className="cs-industry-chip">{story.industry}</span>
                <span className="cs-size-chip">{story.size}</span>
              </div>
              <h1 className="cs-hero-name">{story.name}</h1>
              <p className="cs-hero-kpi"><strong>{story.headlineKpi}</strong></p>
            </div>
          </section>

          {/* 2. Editorial pull-quote */}
          <div className="cs-section-label">Quote</div>
          <section className="cs-pull">
            <p className="cs-pull-text">&ldquo;{story.heroPullQuote}&rdquo;</p>
            <div className="cs-pull-author">— {story.heroQuoteAuthor}</div>
          </section>

          {/* 3. The setup */}
          <div className="cs-section-label">The setup</div>
          <section className="cs-story">
            {story.setupParagraphs.map((p, i) => <p key={i}>{p}</p>)}
          </section>

          {/* 4. Workflow showcase */}
          <div className="cs-section-label">Signature workflow</div>
          <section className="cs-flow">
            <div className="cs-flow-head">
              <div>
                <h3 className="cs-flow-title">{template?.description ?? story.vertical}</h3>
                <p className="cs-flow-vertical">{template?.name ?? story.workflowId}</p>
              </div>
              <Link href={templateHref} className="cs-flow-cta">
                See the template <ArrowRight size={14} />
              </Link>
            </div>
            <div className="cs-flow-steps">
              {(template?.steps ?? []).slice(0, 5).map((step) => {
                const meta = STEP_KIND_META[step.kind];
                return (
                  <div key={step.id} className="cs-flow-step">
                    <div className="cs-flow-step-icon" style={{ background: meta.color }}>{meta.icon}</div>
                    <div style={{ flex: 1 }}>
                      <p className="cs-flow-step-name">{step.name}</p>
                      <p className="cs-flow-step-desc">{step.description}</p>
                      <div className="cs-flow-step-kind">{meta.label}</div>
                    </div>
                  </div>
                );
              })}
            </div>
            <p className="cs-flow-caption">
              {story.name} runs this workflow daily. The first {Math.min(5, template?.steps.length ?? 5)} steps are shown; the full template includes triggers, approval gates, and downstream tool calls.
            </p>
          </section>

          {/* 5. Metrics block */}
          <div className="cs-section-label">In the numbers</div>
          <section className="cs-metrics">
            {story.metrics.map((m, i) => (
              <div key={i} className="cs-metric">
                <div className="cs-metric-val">{m.value}</div>
                <div className="cs-metric-lbl">{m.label}</div>
              </div>
            ))}
          </section>

          {/* 6. Second quote */}
          <div className="cs-section-label">Another voice</div>
          <section className="cs-second">
            <p className="cs-second-text">&ldquo;{story.secondQuote}&rdquo;</p>
            <div className="cs-second-author">— {story.secondQuoteAuthor}</div>
          </section>

          {/* 7. Final CTA */}
          <section className="cs-final">
            <h2>Want this for your team?</h2>
            <p>Hire your first AI teammate today. Free for the first 14 days.</p>
            <Link href="/auth/signup" className="cs-final-cta">
              Start free <ArrowRight size={16} />
            </Link>
          </section>
        </div>
      </div>
    </>
  );
}
