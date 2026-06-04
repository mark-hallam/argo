import Link from "next/link";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";
import { EnterpriseInquiryForm } from "@/components/marketing/EnterpriseInquiryForm";

/**
 * Contact page — v1.0 minimal version. The enterprise/sales path is now a
 * real structured inquiry form (WS6-2) that POSTs to
 * `POST /api/v1/enterprise-inquiry` (persisted + audited) instead of a bare
 * `mailto:`. The remaining options — general email (press/partnerships),
 * demo workspace link, and the founder's X + LinkedIn handles — are unchanged.
 */
export default function ContactPage() {
  return (
    <>
    <MarketingNav />
    <main
      style={{
        maxWidth: 720,
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
        Contact · Reply within 1 business day
      </p>
      <h1
        style={{
          fontFamily: "var(--font-audiowide)",
          fontSize: 48,
          fontWeight: 400,
          lineHeight: 1.05,
          margin: "0 0 14px",
          backgroundImage: "linear-gradient(135deg, var(--primary), var(--gradient-end))",
          backgroundClip: "text",
          WebkitBackgroundClip: "text",
          color: "transparent",
          WebkitTextFillColor: "transparent",
        }}
      >
        Get in touch.
      </h1>
      <p style={{ fontSize: 18, lineHeight: 1.5, margin: "0 0 48px", color: "var(--muted-foreground)" }}>
        Three ways to reach us. The fastest path depends on what you&apos;re after.
      </p>

      <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "grid", gap: 28 }}>
        <li>
          <h2 style={{ fontSize: 22, fontWeight: 700, margin: "0 0 8px" }}>Enterprise</h2>
          <p style={{ fontSize: 15, lineHeight: 1.55, margin: "0 0 16px", color: "var(--muted-foreground)" }}>
            SSO, audit export, custom verticals, and a negotiated rate. Tell us
            about your team — we reply within one business day.
          </p>
          <EnterpriseInquiryForm source="contact" />
        </li>

        <li>
          <h2 style={{ fontSize: 22, fontWeight: 700, margin: "0 0 8px" }}>Email</h2>
          <p style={{ fontSize: 15, lineHeight: 1.55, margin: "0 0 8px", color: "var(--muted-foreground)" }}>
            For partnerships, press, or anything that doesn&apos;t fit a
            form.
          </p>
          <a
            href="mailto:hello@skycot.com"
            style={{ color: "var(--primary)", fontSize: 17, fontWeight: 600 }}
          >
            hello@skycot.com
          </a>
        </li>

        <li>
          <h2 style={{ fontSize: 22, fontWeight: 700, margin: "0 0 8px" }}>Demo</h2>
          <p style={{ fontSize: 15, lineHeight: 1.55, margin: "0 0 8px", color: "var(--muted-foreground)" }}>
            Try a demo workspace · no email required. You&apos;ll be hiring your
            first teammate in under two minutes.
          </p>
          <Link
            href="/auth/signup"
            style={{ color: "var(--primary)", fontSize: 17, fontWeight: 600 }}
          >
            Open a demo workspace →
          </Link>
        </li>

        <li>
          <h2 style={{ fontSize: 22, fontWeight: 700, margin: "0 0 8px" }}>Founder DM</h2>
          <p style={{ fontSize: 15, lineHeight: 1.55, margin: "0 0 8px", color: "var(--muted-foreground)" }}>
            For thoughts on the product, the audit-first take, or the
            company. Mark replies personally.
          </p>
          <p style={{ margin: 0, fontSize: 17, fontWeight: 600 }}>
            <a
              href="https://x.com/MarkHallam"
              style={{ color: "var(--primary)" }}
              rel="noopener"
            >
              X · @MarkHallam
            </a>
            {" · "}
            <a
              href="https://www.linkedin.com/in/mark-hallam/"
              style={{ color: "var(--primary)" }}
              rel="noopener"
            >
              LinkedIn
            </a>
          </p>
        </li>
      </ul>

      <p style={{ marginTop: 56, fontSize: 14, color: "var(--muted-foreground)" }}>
        <Link href="/landing" style={{ color: "var(--primary)" }}>
          Back to home
        </Link>
      </p>
    </main>
    <MarketingFooter />
    </>
  );
}
