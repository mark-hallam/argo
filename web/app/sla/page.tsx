import Link from "next/link";

/**
 * SLA — public response-time targets, surfaced as a real route so the
 * in-product HelpWidget can link to it (previously pointed at the
 * source-tree path `/docs/skycot/support/sla.md`, which 404s in prod).
 *
 * Content mirrors `docs/skycot/support/sla.md`; keep them aligned when
 * tiers change.
 */
export default function SlaPage() {
  return (
    <main
      style={{
        maxWidth: 760,
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
          fontFamily: '"JetBrains Mono", monospace',
          fontSize: 11,
          letterSpacing: "0.2em",
          textTransform: "uppercase",
          color: "var(--muted-foreground)",
          margin: "0 0 18px",
        }}
      >
        Support · response SLAs
      </p>
      <h1
        style={{
          fontFamily: "Audiowide, Inter, sans-serif",
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
        When you&apos;ll hear back.
      </h1>
      <p style={{ fontSize: 18, lineHeight: 1.5, margin: "0 0 32px", color: "var(--muted-foreground)" }}>
        First-response targets for tickets submitted via the in-product help
        widget or{" "}
        <a href="mailto:support@skycot.com" style={{ color: "var(--primary)", fontWeight: 600 }}>
          support@skycot.com
        </a>
        . Targets are business hours (Mon–Fri, 9–5 UTC) unless the tier
        explicitly says 24/7.
      </p>

      <table
        style={{
          width: "100%",
          borderCollapse: "collapse",
          fontSize: 15,
          marginBottom: 36,
        }}
      >
        <thead>
          <tr style={{ background: "#f3f0ff", textAlign: "left" }}>
            <th style={{ padding: "12px 16px", borderBottom: "1px solid var(--border)" }}>Tier</th>
            <th style={{ padding: "12px 16px", borderBottom: "1px solid var(--border)" }}>
              First response
            </th>
            <th style={{ padding: "12px 16px", borderBottom: "1px solid var(--border)" }}>Coverage</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td style={{ padding: "12px 16px", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>
              Free
            </td>
            <td style={{ padding: "12px 16px", borderBottom: "1px solid var(--border)" }}>48 hours</td>
            <td style={{ padding: "12px 16px", borderBottom: "1px solid var(--border)" }}>Business hours.</td>
          </tr>
          <tr>
            <td style={{ padding: "12px 16px", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>
              Pro
            </td>
            <td style={{ padding: "12px 16px", borderBottom: "1px solid var(--border)" }}>24 hours</td>
            <td style={{ padding: "12px 16px", borderBottom: "1px solid var(--border)" }}>Business hours.</td>
          </tr>
          <tr>
            <td style={{ padding: "12px 16px", fontWeight: 600 }}>Enterprise</td>
            <td style={{ padding: "12px 16px" }}>4 hours</td>
            <td style={{ padding: "12px 16px" }}>
              24/7 for severity 1 + 2 incidents.
            </td>
          </tr>
        </tbody>
      </table>

      <h2 style={{ fontSize: 22, fontWeight: 600, margin: "0 0 10px" }}>
        What counts as a first response
      </h2>
      <p style={{ fontSize: 15, lineHeight: 1.6, margin: "0 0 28px", color: "var(--muted-foreground)" }}>
        A human (or human-supervised) reply from the SKYCOT support team that
        acknowledges the ticket, asks a clarifying question, or proposes a fix.
        Automated acknowledgements do not count.
      </p>

      <h2 style={{ fontSize: 22, fontWeight: 600, margin: "0 0 10px" }}>
        Severity definitions (Enterprise)
      </h2>
      <ul style={{ paddingLeft: 22, margin: "0 0 28px", color: "var(--muted-foreground)", lineHeight: 1.7 }}>
        <li>
          <strong>Severity 1</strong> — production outage or data-integrity incident
          affecting the customer&apos;s tenant.
        </li>
        <li>
          <strong>Severity 2</strong> — significant degradation or workflow
          blocker without a workaround.
        </li>
        <li>
          <strong>Severity 3</strong> — general bug, feature request, or how-to
          question.
        </li>
      </ul>
      <p style={{ fontSize: 15, lineHeight: 1.6, margin: "0 0 28px", color: "var(--muted-foreground)" }}>
        Severity 1 + 2 trigger 24/7 coverage; severity 3 follows business hours.
      </p>

      <h2 style={{ fontSize: 22, fontWeight: 600, margin: "0 0 10px" }}>Escalation</h2>
      <p style={{ fontSize: 15, lineHeight: 1.6, margin: "0 0 32px", color: "var(--muted-foreground)" }}>
        If the SLA target is missed, the ticket is auto-escalated to the
        on-call engineer via the operator&apos;s pager rotation. Enterprise
        contracts may name a dedicated technical contact.
      </p>

      <p style={{ marginTop: 48, fontSize: 14, color: "var(--muted-foreground)" }}>
        Back to{" "}
        <Link href="/help" style={{ color: "var(--primary)" }}>
          Help
        </Link>
        .
      </p>
    </main>
  );
}
