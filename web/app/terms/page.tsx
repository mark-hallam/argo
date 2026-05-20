import Link from "next/link";

/**
 * Terms of service — v1.0 placeholder until full legal review lands.
 * The bullets reflect operating principles (no surprise lock-in,
 * data portability, approval-first defaults) the product actually
 * delivers today.
 */
export default function TermsPage() {
  return (
    <main
      style={{
        maxWidth: 720,
        margin: "0 auto",
        padding: "80px 24px 120px",
        fontFamily:
          'Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif',
        color: "#1e1b4b",
        background: "#fafaff",
        minHeight: "100vh",
      }}
    >
      <p
        style={{
          fontFamily: '"JetBrains Mono", monospace',
          fontSize: 11,
          letterSpacing: "0.2em",
          textTransform: "uppercase",
          color: "#71717a",
          margin: "0 0 18px",
        }}
      >
        Last updated · 2026-05-20
      </p>
      <h1
        style={{
          fontFamily: "Audiowide, Inter, sans-serif",
          fontSize: 48,
          fontWeight: 400,
          lineHeight: 1.05,
          margin: "0 0 14px",
          backgroundImage: "linear-gradient(135deg, #8b5cf6, #ec4899)",
          backgroundClip: "text",
          WebkitBackgroundClip: "text",
          color: "transparent",
          WebkitTextFillColor: "transparent",
        }}
      >
        Terms.
      </h1>
      <p style={{ fontSize: 18, lineHeight: 1.5, margin: "0 0 32px", color: "#52525b" }}>
        How SKYCOT and your team agree to work together. Full legal terms
        follow the public launch; the principles below are operative today.
      </p>

      <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "grid", gap: 20 }}>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>No surprise lock-in.</strong>
          You can export every workflow, every audit row, and every memory
          entry as JSON or TOML. We will never hold your data hostage.
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>Cancel anytime.</strong>
          Month-to-month plans cancel at the end of the current billing
          period. Annual plans pro-rate refunds on the unused months.
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>You own your content.</strong>
          We don&apos;t claim ownership of workflows, memories, audit history, or
          anything your teammates produce. Full IP retention on your side.
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>Approval-first by default.</strong>
          New teammates ship in approval-required mode. Trust elevation is
          opt-in, reversible, and logged. We never auto-escalate worker
          permissions without your written go-ahead.
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>Service availability.</strong>
          Hosted SKYCOT targets 99.9% uptime; self-hosted SKYCOT is fully on
          your infrastructure. Status page:{" "}
          <a href="https://status.skycot.com" style={{ color: "#8b5cf6" }}>
            status.skycot.com
          </a>
          .
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>
            Acceptable use.
          </strong>
          SKYCOT teammates may not be used for fraud, harassment, illegal
          surveillance, or anything that would land you on a regulator&apos;s
          watch list. We reserve the right to suspend accounts that do.
        </li>
      </ul>

      <p style={{ marginTop: 48, fontSize: 14, color: "#71717a" }}>
        Full terms under legal review. Questions?{" "}
        <a href="mailto:legal@skycot.com" style={{ color: "#8b5cf6" }}>
          legal@skycot.com
        </a>
        .{" "}
        <Link href="/landing" style={{ color: "#8b5cf6" }}>
          Back to home
        </Link>
        .
      </p>
    </main>
  );
}
