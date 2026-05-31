import Link from "next/link";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";

const DIFFERENTIATORS = [
  {
    label: "Hash-chained audit",
    body: "Every action lands in a tamper-evident trail that can be verified later.",
  },
  {
    label: "Truth-bounded claims",
    body: "Teammates separate verified facts from unknowns instead of filling gaps with theater.",
  },
  {
    label: "Approval-first channels",
    body: "Sensitive work asks a human before it touches customers, money, or production systems.",
  },
  {
    label: "Persistent teammates",
    body: "Teammates have a name, role, memory, and duties, so teams delegate instead of babysitting jobs.",
  },
] as const;

/**
 * About page — v1.0 placeholder. Brand-restrained editorial copy in
 * the same register as /privacy and /security: what SKYCOT is, who
 * built it, and the load-bearing mission line ("Don't buy another
 * tool. Hire a teammate."). Founder bio + team page deferred.
 */
export default function AboutPage() {
  return (
    <>
      <MarketingNav />
      <article
        style={{
          maxWidth: 820,
          margin: "0 auto",
          padding: "80px 24px 120px",
          fontFamily:
            'Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif',
          color: "var(--foreground)",
          background: "var(--background)",
          minHeight: "100vh",
        }}
      >
        <p
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            letterSpacing: "0.2em",
            textTransform: "uppercase",
            color: "var(--muted-foreground)",
            margin: "0 0 18px",
          }}
        >
          About · Made in Melbourne
        </p>
        <h1
          style={{
            fontFamily: "var(--font-audiowide)",
            fontSize: 48,
            fontWeight: 400,
            lineHeight: 1.05,
            margin: "0 0 14px",
            backgroundImage: "var(--brand-gradient)",
            backgroundClip: "text",
            WebkitBackgroundClip: "text",
            color: "transparent",
            WebkitTextFillColor: "transparent",
          }}
        >
          SKYCOT — built in Melbourne.
        </h1>
        <p style={{ fontSize: 18, lineHeight: 1.5, margin: "0 0 32px", color: "var(--muted-foreground)" }}>
          Don&apos;t buy another tool. Hire a teammate. SKYCOT puts approval-first
          AI teammates inside the software you already use — Slack, email, your
          ticketing system — and ships a full audit trail of every action they
          take.
        </p>

        <h2 style={{ fontSize: 22, fontWeight: 700, margin: "32px 0 14px" }}>What we are</h2>
        <p style={{ fontSize: 15, lineHeight: 1.6, margin: "0 0 18px", color: "var(--muted-foreground)" }}>
          SKYCOT is a single-binary enterprise AI agent platform. Teammates live in
          the channels your team already lives in, ask before they act on
          anything sensitive, and learn from your feedback over time. Every
          decision they make is recorded with a tamper-evident hash chain — so
          you can prove, months later, exactly what happened and why.
        </p>
        <p style={{ fontSize: 15, lineHeight: 1.6, margin: "0 0 18px", color: "var(--muted-foreground)" }}>
          The platform ships with a marketplace of templated teammate roles —
          receptionists, status reporters, project coordinators, sales reps —
          each one configurable, governable, and replaceable without rewriting
          any of your existing systems.
        </p>

        <section
          aria-labelledby="about-differentiators"
          style={{
            margin: "36px 0",
            padding: 18,
            border: "1px solid var(--border)",
            borderRadius: 12,
            background:
              "linear-gradient(135deg, color-mix(in oklch, var(--brand-primary) 7%, transparent), color-mix(in oklch, var(--brand-pink) 6%, transparent)), var(--card)",
          }}
        >
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              letterSpacing: "0.16em",
              textTransform: "uppercase",
              color: "var(--muted-foreground)",
              margin: "0 0 8px",
            }}
          >
            Verified differentiators
          </p>
          <h2 id="about-differentiators" style={{ fontSize: 22, fontWeight: 700, margin: "0 0 16px" }}>
            What makes SKYCOT different
          </h2>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
              gap: 12,
            }}
          >
            {DIFFERENTIATORS.map((item) => (
              <div key={item.label} style={{ padding: 12, borderRadius: 8, background: "var(--card)" }}>
                <h3 style={{ fontSize: 14, fontWeight: 700, margin: "0 0 6px" }}>{item.label}</h3>
                <p style={{ fontSize: 13, lineHeight: 1.5, margin: 0, color: "var(--muted-foreground)" }}>
                  {item.body}
                </p>
              </div>
            ))}
          </div>
        </section>

        <h2 style={{ fontSize: 22, fontWeight: 700, margin: "32px 0 14px" }}>Who built it</h2>
        <p style={{ fontSize: 15, lineHeight: 1.6, margin: "0 0 18px", color: "var(--muted-foreground)" }}>
          SKYCOT is built in Melbourne, Australia by{" "}
          <a href="https://markhallam.com.au" style={{ color: "var(--primary)" }}>
            Mark Hallam
          </a>{" "}
          and a small team of engineers who&apos;ve shipped audit-grade software
          into regulated industries before. We&apos;re building the platform we
          wanted when we ran AI inside our own businesses and couldn&apos;t find
          anything that took approvals, audit, and isolation seriously.
        </p>

        <h2 style={{ fontSize: 22, fontWeight: 700, margin: "32px 0 14px" }}>The mission</h2>
        <p style={{ fontSize: 15, lineHeight: 1.6, margin: "0 0 32px", color: "var(--muted-foreground)" }}>
          Most AI tools are toys you have to babysit. SKYCOT is built so the AI
          can do the work, ask when it&apos;s unsure, and leave a paper trail your
          compliance team can sign off on. The goal isn&apos;t a chatbot — it&apos;s a
          teammate that earns your trust the same way a new hire does.
        </p>

        <p style={{ marginTop: 40, fontSize: 14, color: "var(--muted-foreground)" }}>
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
        </p>

        <p style={{ marginTop: 32, fontSize: 14, color: "var(--muted-foreground)" }}>
          <Link href="/landing" style={{ color: "var(--primary)" }}>
            Back to home
          </Link>
        </p>
      </article>
      <MarketingFooter />
    </>
  );
}
