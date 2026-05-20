"use client";

import Link from "next/link";
import { ArrowRight, Check, Shield, MessageSquareText, Sparkles, GitBranch, Users2 } from "lucide-react";

/**
 * Public marketing landing — `/landing`, exempt from auth middleware.
 *
 * The pre-signup surface. Brand-loud-marketing register per the
 * locked D3-with-audiowide mockup at
 * docs/skycot/design-explorations/2026-05-18/marketing-landing/
 * option-D3-with-audiowide.html.
 *
 * Hero: "Don't buy another tool. Hire a teammate. (Seriously.)"
 * Hero CTA: Sign up | Watch 90-second demo
 * Sections: How it works · Customer logos · Feature highlights ·
 *           Comparison teaser → /compare · Final CTA
 *
 * V1 ships static copy; CMS-driven testimonials + case studies arrive
 * in a follow-up brief.
 */

const FEATURES = [
  {
    icon: <Users2 size={22} />,
    title: "Named teammates, not tools",
    blurb:
      "Each AI teammate has a name, a personality, and a job. They show up in Slack like a colleague.",
  },
  {
    icon: <Shield size={22} />,
    title: "Asks before it acts",
    blurb:
      "Every risky action waits for your yes. Approval-first by default. Trust ratchets up over time.",
  },
  {
    icon: <MessageSquareText size={22} />,
    title: "Edit workflows in chat",
    blurb:
      "Describe the change in plain English. Your teammate proposes the diff, you approve, it ships.",
  },
  {
    icon: <GitBranch size={22} />,
    title: "Versioned + reversible",
    blurb:
      "Every workflow change versioned like code. Side-by-side diff. Rollback shipping next.",
  },
  {
    icon: <Sparkles size={22} />,
    title: "Multi-channel native",
    blurb:
      "Slack, iMessage, email, web inbox — all wire in the same approval flow. No extra setup.",
  },
  {
    icon: <Check size={22} />,
    title: "Audit-first, enterprise-grade",
    blurb:
      "Forensic-grade audit chain on every action. Per-action role permissions. Encryption-at-rest by default.",
  },
];

const LOGOS = [
  "Northwind Commerce",
  "Acmeco Retail",
  "Skyline Engineering",
  "Apex Recruiting",
  "Beacon Health Ops",
  "Maple Finance",
  "Polaris Marketing",
  "Zenith Support",
  "Harbour Legal",
  "Meridian Logistics",
  "Atlas Manufacturing",
  "Lumen Education",
];

const TESTIMONIALS = [
  {
    quote: "Mariah replied to 12,400 buyers our human team didn't have to. She knows when to escalate.",
    name: "Daniela Reyes",
    role: "Head of CX",
    company: "Northwind Commerce",
  },
  {
    quote: "Approval-first was the deal-breaker. Every action audit-logged, every decision traced. Our compliance team finally relaxed.",
    name: "Marcus Lin",
    role: "Chief Compliance Officer",
    company: "Beacon Health Ops",
  },
  {
    quote: "Patrick chases status across 14 client engagements at once. He remembers what every project needs.",
    name: "Aisha Patel",
    role: "Operations Director",
    company: "Polaris Marketing",
  },
];

const HOW = [
  {
    n: "01",
    title: "Describe the job",
    blurb:
      "In plain English, tell SKYCOT what you'd hire someone for. 'I need someone to screen inbound leads, qualify them, and book interviews.'",
  },
  {
    n: "02",
    title: "Meet your new teammate",
    blurb:
      "SKYCOT spins up a named worker (Priya, Jamal, Mariah, etc.) with the right skills, joins your Slack, and introduces themselves.",
  },
  {
    n: "03",
    title: "Approve their first decisions",
    blurb:
      "Your teammate proposes, you approve. Their trust grade rises as they ship correct decisions. Eventually they earn full autonomy.",
  },
  {
    n: "04",
    title: "Watch the audit chain",
    blurb:
      "Every action, every approval, every decision — fully audit-logged with worker, action, cost, latency, and outcome.",
  },
];

export default function MarketingLandingPage() {
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .land {
          --purple: #8b5cf6;
          --pink: #ec4899;
          --coral: #ff6b6b;
          --teal: #14b8a6;
          --ink: #1e1b4b;
          --ink-2: #3f3f46;
          --ink-3: #71717a;
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
        .land::before {
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
        .land::after {
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
        .land-top,
        .land-stage {
          position: relative;
          z-index: 1;
        }
        .land-top {
          padding: 22px 64px;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .land-brand {
          display: flex;
          align-items: center;
          gap: 12px;
        }
        .land-brand-mark {
          width: 28px;
          height: 28px;
          border-radius: 7px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          box-shadow: 0 4px 12px -2px rgba(139, 92, 246, 0.3);
        }
        .land-brand-name {
          font-family: "Audiowide", sans-serif;
          font-size: 16px;
          letter-spacing: 0.08em;
        }
        .land-nav {
          display: flex;
          gap: 28px;
          font-size: 14px;
          font-weight: 500;
          align-items: center;
        }
        .land-nav a {
          color: var(--ink-2);
          text-decoration: none;
        }
        .land-nav a:hover {
          color: var(--purple);
        }
        .land-for-wrap {
          position: relative;
        }
        .land-for-trigger {
          background: none;
          border: none;
          padding: 0;
          font: inherit;
          color: var(--ink-2);
          cursor: pointer;
          display: inline-flex;
          gap: 6px;
          align-items: center;
        }
        .land-for-trigger:hover {
          color: var(--purple);
        }
        .land-for-wrap:hover .land-for-menu,
        .land-for-wrap:focus-within .land-for-menu {
          display: grid;
        }
        .land-for-menu {
          display: none;
          position: absolute;
          top: calc(100% + 12px);
          left: 50%;
          transform: translateX(-50%);
          background: white;
          border: 1px solid var(--line);
          border-radius: 14px;
          padding: 10px;
          min-width: 280px;
          grid-template-columns: 1fr;
          gap: 4px;
          box-shadow: 0 18px 40px -16px rgba(30, 27, 75, 0.18);
          z-index: 50;
        }
        .land-for-item {
          padding: 10px 12px;
          border-radius: 10px;
          text-decoration: none;
          color: var(--ink);
        }
        .land-for-item:hover {
          background: rgba(139, 92, 246, 0.06);
        }
        .land-for-item-label {
          font-weight: 600;
          font-size: 14px;
          margin: 0;
        }
        .land-for-item-tag {
          margin: 2px 0 0;
          font-size: 12.5px;
          color: var(--ink-3);
        }
        .land-top-right {
          display: flex;
          gap: 12px;
          align-items: center;
        }
        .land-signin {
          font-size: 14px;
          color: var(--ink);
          text-decoration: none;
          font-weight: 600;
        }
        .land-cta-pill {
          padding: 9px 18px;
          border-radius: 999px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-size: 14px;
          font-weight: 700;
          text-decoration: none;
          box-shadow: 0 6px 18px -4px rgba(139, 92, 246, 0.45);
        }
        .land-hero {
          padding: 80px 64px 64px;
          display: grid;
          grid-template-columns: 1.1fr 1fr;
          gap: 64px;
          align-items: center;
          max-width: 1280px;
          margin: 0 auto;
        }
        @media (max-width: 900px) {
          .land-hero {
            grid-template-columns: 1fr;
            padding: 48px 24px 32px;
          }
          .land-top {
            padding: 16px 24px;
          }
          .land-nav {
            display: none;
          }
        }
        .land-hero-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 700;
          padding: 6px 14px;
          border-radius: 999px;
          background: rgba(139, 92, 246, 0.1);
          color: var(--purple);
          margin-bottom: 24px;
        }
        .land-hero-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--coral);
        }
        .land-hero-h1 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 68px;
          line-height: 1;
          font-weight: 400;
          color: var(--ink);
          margin: 0 0 24px;
          letter-spacing: 0;
        }
        .land-hero-h1 strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .land-hero-sub {
          font-size: 18px;
          font-weight: 300;
          color: var(--ink-2);
          line-height: 1.55;
          margin: 0 0 32px;
          max-width: 520px;
        }
        .land-hero-actions {
          display: flex;
          gap: 14px;
          align-items: center;
        }
        .land-btn-primary {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 14px 24px;
          border-radius: 12px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-size: 15px;
          font-weight: 700;
          border: none;
          cursor: pointer;
          text-decoration: none;
          box-shadow: 0 8px 24px -6px rgba(139, 92, 246, 0.45);
        }
        .land-btn-secondary {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 14px 22px;
          border-radius: 12px;
          background: transparent;
          color: var(--ink);
          font-size: 15px;
          font-weight: 600;
          border: 1px solid var(--line);
          cursor: pointer;
          text-decoration: none;
        }
        .land-trust-strip {
          margin-top: 36px;
          display: flex;
          gap: 32px;
          flex-wrap: wrap;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.06em;
          text-transform: uppercase;
          font-weight: 600;
        }
        .land-trust-item {
          display: inline-flex;
          gap: 6px;
          align-items: center;
        }
        .land-trust-dot {
          width: 5px;
          height: 5px;
          border-radius: 50%;
          background: var(--teal);
        }
        .land-hero-art {
          position: relative;
          aspect-ratio: 1;
          max-width: 520px;
          margin: 0 auto;
        }
        .land-hero-art::before {
          content: "";
          position: absolute;
          inset: 0;
          background: radial-gradient(
            circle at center,
            rgba(139, 92, 246, 0.18),
            rgba(236, 72, 153, 0.12) 40%,
            transparent 70%
          );
          border-radius: 50%;
        }
        .land-art-bubble {
          position: absolute;
          background: white;
          border: 1px solid var(--line);
          border-radius: 14px;
          padding: 14px 18px;
          font-size: 13.5px;
          color: var(--ink);
          box-shadow: 0 10px 30px -10px rgba(139, 92, 246, 0.18);
          font-weight: 500;
          /* Wave-6 e2e fix (2026-05-21): bubbles were collapsing to
             ~90px wide and breaking text vertically — and overlapping
             the SKYCOT logo in the hero centre. Pin min-width + max
             so the text stays on 1-2 lines and the bubbles clear the
             centre logo with breathing room. */
          min-width: 220px;
          max-width: 280px;
        }
        .land-art-bubble::before {
          content: "";
          position: absolute;
          top: 50%;
          right: 100%;
          width: 30px;
          height: 1px;
          background: linear-gradient(90deg, transparent, var(--line));
        }
        .land-art-center {
          position: absolute;
          top: 50%;
          left: 50%;
          transform: translate(-50%, -50%);
          width: 88px;
          height: 88px;
          border-radius: 22px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          display: grid;
          place-items: center;
          color: white;
          font-family: "Audiowide", sans-serif;
          font-size: 11px;
          letter-spacing: 0.16em;
          box-shadow: 0 20px 50px -10px rgba(139, 92, 246, 0.5);
        }
        .land-art-bubble-1 {
          top: 16%;
          left: 8%;
          font-size: 12.5px;
        }
        .land-art-bubble-2 {
          top: 22%;
          right: 6%;
          font-size: 12.5px;
        }
        .land-art-bubble-3 {
          bottom: 28%;
          left: 6%;
          font-size: 12.5px;
        }
        .land-art-bubble-4 {
          bottom: 14%;
          right: 12%;
          font-size: 12.5px;
        }
        /* Wave-9 window-resize fix (2026-05-21): the wave-6 fix
           clamped bubbles min-width: 220px which keeps them readable,
           but at the intermediate viewport range (700–890px) bubbles
           still overlap the centre logo. Stack vertically (2 cols,
           non-overlapping rows) at this band; full grid at 891px+. */
        @media (max-width: 890px) {
          .land-art-bubble {
            min-width: 0 !important;
            max-width: 44% !important;
            font-size: 11.5px !important;
            padding: 10px 14px !important;
          }
          .land-art-bubble-1 { top: 4%; left: 4%; }
          .land-art-bubble-2 { top: 4%; right: 4%; }
          .land-art-bubble-3 { bottom: 4%; left: 4%; }
          .land-art-bubble-4 { bottom: 4%; right: 4%; }
        }
        @media (max-width: 540px) {
          .land-hero-art {
            display: none;
          }
        }
        .land-section-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          margin: 0 0 10px;
          text-align: center;
        }
        .land-section-title {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 40px;
          font-weight: 400;
          color: var(--ink);
          text-align: center;
          margin: 0 0 48px;
          letter-spacing: 0;
          line-height: 1.1;
        }
        .land-section-title strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .land-section {
          padding: 80px 64px;
          max-width: 1180px;
          margin: 0 auto;
        }
        @media (max-width: 700px) {
          .land-section {
            padding: 48px 24px;
          }
          .land-section-title {
            font-size: 30px;
          }
          .land-hero-h1 {
            font-size: 44px;
          }
        }
        .land-feature-grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 16px;
        }
        @media (max-width: 900px) {
          .land-feature-grid {
            grid-template-columns: 1fr;
          }
        }
        .land-feature {
          background: white;
          border: 1px solid var(--line);
          border-radius: 14px;
          padding: 22px;
          transition: all 0.18s;
        }
        .land-feature:hover {
          border-color: rgba(139, 92, 246, 0.3);
          transform: translateY(-2px);
          box-shadow: 0 12px 24px -10px rgba(139, 92, 246, 0.15);
        }
        .land-feature-icon {
          width: 40px;
          height: 40px;
          border-radius: 10px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          margin-bottom: 14px;
        }
        .land-feature-title {
          font-weight: 600;
          font-size: 17px;
          color: var(--ink);
          margin: 0 0 6px;
        }
        .land-feature-blurb {
          font-size: 13.5px;
          color: var(--ink-2);
          line-height: 1.5;
          margin: 0;
        }
        .land-how-grid {
          display: grid;
          grid-template-columns: repeat(2, 1fr);
          gap: 32px;
          max-width: 960px;
          margin: 0 auto;
        }
        @media (max-width: 720px) {
          .land-how-grid {
            grid-template-columns: 1fr;
          }
        }
        .land-how-step {
          padding: 8px 0;
        }
        .land-how-n {
          font-family: "Audiowide", sans-serif;
          font-size: 36px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
          line-height: 1;
          margin-bottom: 12px;
          font-weight: 400;
        }
        .land-how-title {
          font-weight: 600;
          font-size: 19px;
          color: var(--ink);
          margin: 0 0 8px;
        }
        .land-how-blurb {
          font-size: 14.5px;
          color: var(--ink-2);
          line-height: 1.55;
          margin: 0;
          max-width: 420px;
        }
        .land-social {
          padding: 32px 0 72px;
          max-width: 1320px;
          margin: 0 auto;
          overflow: hidden;
        }
        .land-social-strip {
          display: flex;
          gap: 14px;
          padding: 6px 64px 28px;
          overflow-x: auto;
          scrollbar-width: none;
        }
        .land-social-strip::-webkit-scrollbar {
          display: none;
        }
        @media (max-width: 700px) {
          .land-social-strip {
            padding: 6px 24px 24px;
          }
        }
        .land-social-card {
          flex: 0 0 auto;
          display: flex;
          align-items: center;
          gap: 10px;
          padding: 14px 18px;
          background: white;
          border: 1px solid var(--line);
          border-radius: 12px;
          font-family: "JetBrains Mono", monospace;
          font-size: 12px;
          color: var(--ink);
          font-weight: 600;
          letter-spacing: 0.04em;
          white-space: nowrap;
        }
        .land-social-logo {
          width: 26px;
          height: 26px;
          border-radius: 6px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          font-family: "Audiowide", sans-serif;
          font-size: 11px;
          letter-spacing: 0.02em;
        }
        .land-quote-grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 18px;
          padding: 0 64px;
          max-width: 1240px;
          margin: 36px auto 0;
        }
        @media (max-width: 900px) {
          .land-quote-grid {
            grid-template-columns: 1fr;
            padding: 0 24px;
          }
        }
        .land-quote-card {
          background: white;
          border: 1px solid var(--line);
          border-radius: 16px;
          padding: 26px;
          display: flex;
          flex-direction: column;
          gap: 16px;
          transition: all 0.18s;
        }
        .land-quote-card:hover {
          border-color: rgba(139, 92, 246, 0.3);
          transform: translateY(-2px);
          box-shadow: 0 14px 28px -12px rgba(139, 92, 246, 0.18);
        }
        .land-quote-mark {
          font-family: "Audiowide", sans-serif;
          font-size: 36px;
          line-height: 1;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .land-quote-text {
          font-size: 15px;
          color: var(--ink);
          line-height: 1.55;
          margin: 0;
          font-weight: 400;
        }
        .land-quote-byline {
          display: flex;
          gap: 12px;
          align-items: center;
          padding-top: 14px;
          border-top: 1px solid var(--line);
        }
        .land-quote-avatar {
          width: 36px;
          height: 36px;
          border-radius: 50%;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          font-family: "Audiowide", sans-serif;
          font-size: 13px;
          flex-shrink: 0;
        }
        .land-quote-name {
          font-weight: 700;
          font-size: 13.5px;
          color: var(--ink);
          margin: 0;
        }
        .land-quote-role {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.1em;
          text-transform: uppercase;
          color: var(--ink-3);
          margin-top: 2px;
        }
        .land-compare-band {
          margin: 32px 64px 0;
          padding: 56px 48px;
          border-radius: 24px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899, #ff6b6b);
          color: white;
          text-align: center;
          position: relative;
          overflow: hidden;
        }
        @media (max-width: 700px) {
          .land-compare-band {
            margin: 16px 12px 0;
            padding: 36px 24px;
          }
        }
        .land-compare-band::before {
          content: "";
          position: absolute;
          inset: 0;
          background: linear-gradient(
            115deg,
            transparent 30%,
            rgba(255, 255, 255, 0.14) 50%,
            transparent 70%
          );
          transform: translateX(-100%);
          animation: land-shine 6s ease-in-out infinite;
          pointer-events: none;
        }
        @keyframes land-shine {
          0%, 80%, 100% {
            transform: translateX(-100%);
          }
          90% {
            transform: translateX(100%);
          }
        }
        .land-compare-band h2 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 36px;
          font-weight: 400;
          color: white;
          margin: 0 0 14px;
          position: relative;
        }
        .land-compare-band p {
          font-size: 16px;
          color: rgba(255, 255, 255, 0.92);
          margin: 0 0 26px;
          max-width: 580px;
          margin-left: auto;
          margin-right: auto;
          position: relative;
        }
        .land-compare-cta {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 14px 28px;
          border-radius: 12px;
          background: white;
          color: #8b5cf6;
          font-size: 15px;
          font-weight: 700;
          text-decoration: none;
          position: relative;
        }
        .land-final {
          padding: 100px 64px 80px;
          text-align: center;
        }
        .land-final h2 {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 48px;
          font-weight: 400;
          margin: 0 0 18px;
          color: var(--ink);
        }
        .land-final h2 strong {
          font-weight: 400;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .land-foot {
          padding: 32px 64px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.12em;
          text-transform: uppercase;
          font-weight: 600;
          text-align: center;
          border-top: 1px solid var(--line);
        }
      `}</style>
      <div className="land">
        <header className="land-top">
          <div className="land-brand">
            <div className="land-brand-mark" />
            <div className="land-brand-name">SKYCOT</div>
          </div>
          <nav className="land-nav">
            <a href="#how">How it works</a>
            <a href="#features">Features</a>
            <span className="land-for-wrap">
              <button className="land-for-trigger" type="button" aria-haspopup="true">
                For <ArrowRight size={12} style={{ transform: "rotate(90deg)" }} />
              </button>
              <div className="land-for-menu" role="menu">
                {[
                  { slug: "sales", label: "Sales", tagline: "Qualify leads, book demos." },
                  { slug: "recruiting", label: "Recruiting", tagline: "Screen, schedule, reference." },
                  { slug: "support", label: "Support", tagline: "Triage tickets, draft replies." },
                  { slug: "finance", label: "Finance", tagline: "Extract, reconcile, flag." },
                  { slug: "marketing", label: "Marketing", tagline: "Brand QA, enrich, digest." },
                  { slug: "ops", label: "Ops", tagline: "Status, sync, audit." },
                ].map((v) => (
                  <Link key={v.slug} href={`/for/${v.slug}`} className="land-for-item" role="menuitem">
                    <p className="land-for-item-label">{v.label}</p>
                    <p className="land-for-item-tag">{v.tagline}</p>
                  </Link>
                ))}
              </div>
            </span>
            <Link href="/compare">Compare</Link>
            <Link href="/customers">Customers</Link>
            <Link href="/pricing">Pricing</Link>
          </nav>
          <div className="land-top-right">
            <Link href="/auth/signin" className="land-signin">
              Sign in
            </Link>
            <Link href="/auth/signup" className="land-cta-pill">
              Hire your first teammate
            </Link>
          </div>
        </header>

        {/* Hero */}
        <section className="land-hero">
          <div>
            <div className="land-hero-eyebrow">
              <span className="land-hero-eyebrow-dot" />
              v1.0 shipped · Hiring our first 100 teams
            </div>
            <h1 className="land-hero-h1">
              Don&apos;t buy another tool.
              <br />
              Hire a <strong>teammate</strong>.<br />
              (Seriously.)
            </h1>
            <p className="land-hero-sub">
              SKYCOT compiles plain-English job descriptions into AI teammates that live in Slack,
              learn from your feedback, and ask for approval before they act. Audit-first,
              single-binary, enterprise-grade.
            </p>
            <div className="land-hero-actions">
              <Link href="/auth/signup" className="land-btn-primary">
                Hire your first teammate <ArrowRight size={16} />
              </Link>
              <a href="#how" className="land-btn-secondary">
                Watch a 90-second demo
              </a>
            </div>
            <div className="land-trust-strip">
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                NO CREDIT CARD
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                SOC 2-READY AUDIT TRAIL
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                ISO 27001 CONTROLS SUPPORTED
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                GDPR · HIPAA WORKFLOWS
              </span>
              <span className="land-trust-item">
                <span className="land-trust-dot" />
                2,847 teams this month
              </span>
            </div>
          </div>

          <div className="land-hero-art">
            <div className="land-art-bubble land-art-bubble-1">
              <strong style={{ color: "#8b5cf6" }}>Mariah</strong>: 3 listings live, 2 buyers waiting on you.
            </div>
            <div className="land-art-bubble land-art-bubble-2">
              <strong style={{ color: "#ec4899" }}>Priya</strong>: 8 candidates screened today.
            </div>
            <div className="land-art-bubble land-art-bubble-3">
              <strong style={{ color: "#0891b2" }}>Jamal</strong>: 12 tickets resolved, median reply 4m.
            </div>
            <div className="land-art-bubble land-art-bubble-4">
              <strong style={{ color: "#14b8a6" }}>Nori</strong>: Q2 update draft ready for review.
            </div>
            <div className="land-art-center">SKYCOT</div>
          </div>
        </section>

        {/* How it works */}
        <section id="how" className="land-section">
          <p className="land-section-label">HOW IT WORKS</p>
          <h2 className="land-section-title">
            From plain English to a <strong>working teammate</strong> in four minutes.
          </h2>
          <div className="land-how-grid">
            {HOW.map((s) => (
              <div key={s.n} className="land-how-step">
                <div className="land-how-n">{s.n}</div>
                <h3 className="land-how-title">{s.title}</h3>
                <p className="land-how-blurb">{s.blurb}</p>
              </div>
            ))}
          </div>
        </section>

        {/* Features */}
        <section id="features" className="land-section">
          <p className="land-section-label">WHY SKYCOT</p>
          <h2 className="land-section-title">
            Why your team will <strong>actually use them</strong>.
          </h2>
          <div className="land-feature-grid">
            {FEATURES.map((f) => (
              <div key={f.title} className="land-feature">
                <div className="land-feature-icon">{f.icon}</div>
                <h3 className="land-feature-title">{f.title}</h3>
                <p className="land-feature-blurb">{f.blurb}</p>
              </div>
            ))}
          </div>
        </section>

        {/* Social proof — logos + testimonials */}
        <section className="land-social">
          <p className="land-section-label">TRUSTED BY TEAMS LIKE YOURS</p>
          <h2 className="land-section-title" style={{ marginBottom: 28 }}>
            Already shipping with <strong>SKYCOT</strong>.
          </h2>
          <div className="land-social-strip">
            {[...LOGOS, ...LOGOS].map((logo, i) => (
              <div key={`${logo}-${i}`} className="land-social-card">
                <span className="land-social-logo">{logo.split(" ").map((w) => w[0]).join("").slice(0, 2)}</span>
                <span>{logo}</span>
              </div>
            ))}
          </div>
          <div className="land-quote-grid">
            {TESTIMONIALS.map((t) => (
              <article key={t.name} className="land-quote-card">
                <div className="land-quote-mark">&ldquo;</div>
                <p className="land-quote-text">{t.quote}</p>
                <div className="land-quote-byline">
                  <div className="land-quote-avatar">
                    {t.name.split(" ").map((n) => n[0]).join("").slice(0, 2)}
                  </div>
                  <div>
                    <div className="land-quote-name">{t.name}</div>
                    <div className="land-quote-role">{t.role} · {t.company}</div>
                  </div>
                </div>
              </article>
            ))}
          </div>
          <div style={{ textAlign: "center", marginTop: 36 }}>
            <Link
              href="/customers"
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 8,
                padding: "12px 22px",
                borderRadius: 12,
                background: "white",
                border: "1px solid var(--line, #ece9f5)",
                color: "var(--ink, #1e1b4b)",
                fontSize: 14.5,
                fontWeight: 600,
                textDecoration: "none",
              }}
            >
              See all customers <ArrowRight size={15} />
            </Link>
          </div>
        </section>

        {/* Integration logos */}
        <section className="land-section">
          <p className="land-section-label">CONNECTS TO YOUR STACK</p>
          <h2 className="land-section-title">
            Already where your <strong>work happens</strong>.
          </h2>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))",
              gap: 14,
              maxWidth: 900,
              margin: "0 auto",
            }}
          >
            {[
              "Slack",
              "Gmail",
              "Outlook",
              "Zendesk",
              "Intercom",
              "Salesforce",
              "HubSpot",
              "QuickBooks",
              "Xero",
              "NetSuite",
              "Greenhouse",
              "Lever",
              "Workday",
              "Notion",
              "Linear",
              "GitHub",
              "Stripe",
              "Shopify",
            ].map((name) => (
              <div
                key={name}
                style={{
                  padding: "16px 18px",
                  border: "1px solid var(--line, #ece9f5)",
                  borderRadius: 12,
                  background: "white",
                  textAlign: "center",
                  fontFamily: "Inter, sans-serif",
                  fontSize: 13.5,
                  fontWeight: 600,
                  color: "var(--ink, #1e1b4b)",
                }}
              >
                {name}
              </div>
            ))}
          </div>
        </section>

        {/* Compare teaser band */}
        <section className="land-compare-band">
          <h2>How do we stack up vs Zapier, n8n, Crew.ai, Lindy?</h2>
          <p>
            Side-by-side, dimension-by-dimension. Spoiler: 8 of 13 rows are clean wins. Some
            are close. Some we&apos;ve decided not to compete on.
          </p>
          <Link href="/compare" className="land-compare-cta">
            See the comparison <ArrowRight size={16} />
          </Link>
        </section>

        {/* Final CTA */}
        <section className="land-final">
          <h2>
            Your first teammate is <strong>four minutes away</strong>.
          </h2>
          <Link href="/auth/signup" className="land-btn-primary">
            Hire your first teammate <ArrowRight size={16} />
          </Link>
        </section>

        <footer
          className="land-foot"
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 18,
            alignItems: "center",
            padding: "32px 64px",
          }}
        >
          <div style={{ display: "flex", gap: 24, flexWrap: "wrap", justifyContent: "center" }}>
            <a href="/landing" style={{ color: "inherit", textDecoration: "none" }}>
              SKYCOT
            </a>
            <Link href="/compare" style={{ color: "inherit", textDecoration: "none" }}>
              Compare
            </Link>
            <Link href="/customers" style={{ color: "inherit", textDecoration: "none" }}>
              Customers
            </Link>
            <Link href="/pricing" style={{ color: "inherit", textDecoration: "none" }}>
              Pricing
            </Link>
            <a href="https://github.com/mark-hallam/skycot-packs" style={{ color: "inherit", textDecoration: "none" }}>
              GitHub
            </a>
            <Link href="/docs" style={{ color: "inherit", textDecoration: "none" }}>
              Docs
            </Link>
            <Link href="/privacy" style={{ color: "inherit", textDecoration: "none" }}>
              Privacy
            </Link>
            <Link href="/terms" style={{ color: "inherit", textDecoration: "none" }}>
              Terms
            </Link>
            <Link href="/security" style={{ color: "inherit", textDecoration: "none" }}>
              Security
            </Link>
            <a href="mailto:hello@skycot.com" style={{ color: "inherit", textDecoration: "none" }}>
              Contact
            </a>
          </div>
          <div>
            © 2026 SKYCOT · Made in Melbourne · skycot.com
          </div>
        </footer>
      </div>
    </>
  );
}
