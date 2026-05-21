"use client";

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
      <style jsx global>{`
        .cs {
          --purple: #8b5cf6;
          --pink: #ec4899;
          --coral: #ff6b6b;
          --teal: #14b8a6;
          --ink: #1e1b4b;
          --ink-2: #3f3f46;
          --ink-3: #52525b;
          --bg: #fafaff;
          --line: #ece9f5;
          font-family: "Inter", sans-serif;
          background: var(--bg);
          color: var(--ink);
          min-height: 100vh;
          position: relative;
          letter-spacing: -0.005em;
          -webkit-font-smoothing: antialiased;
        }
        .cs::before {
          content: "";
          position: fixed;
          top: -20%;
          right: -10%;
          width: 700px;
          height: 700px;
          background: radial-gradient(circle, rgba(139, 92, 246, 0.13) 0%, transparent 70%);
          z-index: 0;
          pointer-events: none;
        }
        .cs::after {
          content: "";
          position: fixed;
          bottom: -30%;
          left: -10%;
          width: 800px;
          height: 800px;
          background: radial-gradient(circle, rgba(236, 72, 153, 0.1) 0%, transparent 70%);
          z-index: 0;
          pointer-events: none;
        }
        .cs-top { padding: 22px 64px; display: flex; align-items: center; justify-content: space-between; position: relative; z-index: 1; }
        .cs-stage { position: relative; z-index: 1; max-width: 920px; margin: 0 auto; padding: 0 32px 80px; }
        @media (max-width: 700px) {
          .cs-top { padding: 16px 24px; }
          .cs-stage { padding: 0 18px 64px; }
        }
        .cs-brand { display: flex; align-items: center; gap: 12px; text-decoration: none; color: inherit; }
        .cs-brand-mark { width: 28px; height: 28px; border-radius: 7px; background: linear-gradient(135deg, #8b5cf6, #ec4899); box-shadow: 0 4px 12px -2px rgba(139, 92, 246, 0.3); }
        .cs-brand-name { font-family: "Audiowide", sans-serif; font-size: 16px; letter-spacing: 0.08em; }
        .cs-top-right { display: flex; gap: 12px; align-items: center; }
        .cs-signin { font-size: 14px; color: var(--ink); text-decoration: none; font-weight: 600; }
        .cs-cta-pill { padding: 9px 18px; border-radius: 999px; background: linear-gradient(135deg, #8b5cf6, #ec4899); color: white; font-size: 14px; font-weight: 700; text-decoration: none; box-shadow: 0 6px 18px -4px rgba(139, 92, 246, 0.45); }

        .cs-back { display: inline-flex; align-items: center; gap: 6px; font-family: "JetBrains Mono", monospace; font-size: 11px; color: var(--ink-3); letter-spacing: 0.18em; text-transform: uppercase; font-weight: 700; text-decoration: none; margin: 32px 0 24px; }
        .cs-back:hover { color: var(--purple); }

        .cs-hero { padding: 36px 32px; border-radius: 22px; background: linear-gradient(135deg, rgba(139, 92, 246, 0.06), rgba(236, 72, 153, 0.04)); border: 1px solid var(--line); display: flex; gap: 28px; align-items: center; margin-bottom: 32px; }
        @media (max-width: 700px) { .cs-hero { flex-direction: column; align-items: flex-start; padding: 28px 22px; gap: 18px; } }
        .cs-hero-logo { width: 88px; height: 88px; border-radius: 18px; display: grid; place-items: center; color: white; font-family: "Audiowide", sans-serif; font-size: 26px; letter-spacing: 0.04em; flex-shrink: 0; box-shadow: 0 10px 22px -10px rgba(0, 0, 0, 0.2); }
        .cs-hero-meta { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; margin-bottom: 12px; }
        .cs-industry-chip { font-family: "JetBrains Mono", monospace; font-size: 11px; letter-spacing: 0.14em; text-transform: uppercase; font-weight: 700; padding: 5px 12px; border-radius: 999px; background: white; border: 1px solid rgba(139, 92, 246, 0.2); color: var(--purple); }
        .cs-size-chip { font-family: "JetBrains Mono", monospace; font-size: 11px; letter-spacing: 0.12em; text-transform: uppercase; font-weight: 600; color: var(--ink-3); }
        .cs-hero-name { font-family: "Audiowide", "Inter", sans-serif; font-size: 36px; line-height: 1.05; font-weight: 400; margin: 0 0 12px; color: var(--ink); }
        .cs-hero-kpi { font-size: 17px; font-weight: 600; color: var(--ink-2); line-height: 1.45; margin: 0; }
        .cs-hero-kpi strong { font-family: "Audiowide", sans-serif; font-weight: 400; background: linear-gradient(135deg, #8b5cf6, #ec4899); -webkit-background-clip: text; background-clip: text; color: transparent; }

        .cs-section-label { font-family: "JetBrains Mono", monospace; font-size: 11px; color: var(--ink-3); letter-spacing: 0.18em; text-transform: uppercase; font-weight: 700; margin: 40px 0 14px; }

        .cs-pull { padding: 36px 32px; border-radius: 22px; background: white; border: 1px solid var(--line); border-left: 4px solid; border-image: linear-gradient(180deg, #8b5cf6, #ec4899) 1; box-shadow: 0 14px 28px -16px rgba(139, 92, 246, 0.18); }
        .cs-pull-text { font-family: "Inter", sans-serif; font-size: 22px; line-height: 1.45; font-weight: 400; color: var(--ink); margin: 0 0 18px; font-style: italic; }
        @media (max-width: 700px) { .cs-pull-text { font-size: 18px; } .cs-pull { padding: 24px 22px; } }
        .cs-pull-author { font-family: "JetBrains Mono", monospace; font-size: 11.5px; letter-spacing: 0.12em; text-transform: uppercase; font-weight: 700; color: var(--purple); }

        .cs-story p { font-size: 16px; line-height: 1.7; color: var(--ink-2); margin: 0 0 18px; }
        .cs-story p:last-child { margin-bottom: 0; }

        .cs-flow { padding: 28px; border-radius: 18px; background: white; border: 1px solid var(--line); }
        .cs-flow-head { display: flex; gap: 12px; align-items: center; justify-content: space-between; margin-bottom: 18px; flex-wrap: wrap; }
        .cs-flow-title { font-family: "Inter", sans-serif; font-size: 16px; font-weight: 700; color: var(--ink); margin: 0; }
        .cs-flow-vertical { font-family: "JetBrains Mono", monospace; font-size: 10.5px; color: var(--ink-3); letter-spacing: 0.14em; text-transform: uppercase; font-weight: 700; margin: 4px 0 0; }
        .cs-flow-cta { display: inline-flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 700; color: var(--purple); text-decoration: none; }
        .cs-flow-steps { display: flex; flex-direction: column; gap: 10px; }
        .cs-flow-step { display: flex; gap: 12px; padding: 12px 14px; border-radius: 12px; background: var(--bg); border: 1px solid var(--line); }
        .cs-flow-step-icon { width: 26px; height: 26px; border-radius: 8px; display: grid; place-items: center; color: white; flex-shrink: 0; }
        .cs-flow-step-name { font-size: 13.5px; font-weight: 700; color: var(--ink); margin: 0 0 3px; }
        .cs-flow-step-desc { font-size: 12.5px; color: var(--ink-2); line-height: 1.5; margin: 0; }
        .cs-flow-step-kind { font-family: "JetBrains Mono", monospace; font-size: 9.5px; color: var(--ink-3); letter-spacing: 0.12em; text-transform: uppercase; font-weight: 700; margin-top: 4px; }
        .cs-flow-caption { font-size: 13px; color: var(--ink-3); line-height: 1.5; margin: 14px 0 0; }

        .cs-metrics { display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; }
        @media (max-width: 640px) { .cs-metrics { grid-template-columns: 1fr; } }
        .cs-metric { padding: 24px; border-radius: 16px; background: linear-gradient(135deg, rgba(139, 92, 246, 0.06), rgba(236, 72, 153, 0.04)); border: 1px solid var(--line); text-align: center; }
        .cs-metric-val { font-family: "Audiowide", sans-serif; font-size: 32px; font-weight: 400; background: linear-gradient(135deg, #8b5cf6, #ec4899); -webkit-background-clip: text; background-clip: text; color: transparent; line-height: 1.05; margin-bottom: 6px; }
        .cs-metric-lbl { font-family: "JetBrains Mono", monospace; font-size: 10.5px; color: var(--ink-3); letter-spacing: 0.14em; text-transform: uppercase; font-weight: 700; }

        .cs-second { padding: 28px 32px; border-radius: 18px; background: white; border: 1px solid var(--line); }
        .cs-second-text { font-size: 18px; line-height: 1.5; color: var(--ink); margin: 0 0 12px; font-style: italic; }
        .cs-second-author { font-family: "JetBrains Mono", monospace; font-size: 11px; letter-spacing: 0.12em; text-transform: uppercase; font-weight: 700; color: var(--purple); }

        .cs-final { margin: 56px 0 0; padding: 56px 40px; border-radius: 24px; background: linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b); color: white; text-align: center; position: relative; overflow: hidden; }
        @media (max-width: 700px) { .cs-final { padding: 40px 24px; } }
        .cs-final::before { content: ""; position: absolute; inset: 0; background: linear-gradient(115deg, transparent 30%, rgba(255, 255, 255, 0.14) 50%, transparent 70%); transform: translateX(-100%); animation: cs-shine 6s ease-in-out infinite; pointer-events: none; }
        @keyframes cs-shine { 0%, 80%, 100% { transform: translateX(-100%); } 90% { transform: translateX(100%); } }
        .cs-final h2 { font-family: "Audiowide", "Inter", sans-serif; font-size: 32px; font-weight: 400; color: white; margin: 0 0 14px; position: relative; }
        .cs-final p { font-size: 16px; color: rgba(255, 255, 255, 0.92); margin: 0 auto 24px; max-width: 520px; position: relative; }
        .cs-final-cta { display: inline-flex; align-items: center; gap: 8px; padding: 14px 28px; border-radius: 12px; background: white; color: #8b5cf6; font-size: 15px; font-weight: 700; text-decoration: none; position: relative; }
      `}</style>
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
