import Link from "next/link";

/**
 * Security page — v1.0 placeholder. Lists the actual security
 * posture (audit chain, capability permissions, encryption-at-rest,
 * isolation, responsible-disclosure inbox) without overclaiming any
 * external certifications that haven't been issued yet.
 */
export default function SecurityPage() {
  return (
    <>
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
        Last updated · 2026-05-20
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
        Security.
      </h1>
      <p style={{ fontSize: 18, lineHeight: 1.5, margin: "0 0 32px", color: "var(--muted-foreground)" }}>
        SKYCOT is built for buyers whose security review is half the sales
        cycle. Here&apos;s the actual posture today — no aspirational claims.
      </p>

      <h2 style={{ fontSize: 22, fontWeight: 700, margin: "32px 0 14px" }}>What ships today</h2>
      <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "grid", gap: 14 }}>
        <li>
          <strong>Audit chain.</strong> Every action is logged with a tamper-evident
          hash chain — verifiable months later via `argo audit verify`.
        </li>
        <li>
          <strong>Workspace isolation.</strong> Every primitive carries a workspace
          stamp; CI guards block any code that could leak across the boundary.
        </li>
        <li>
          <strong>Per-action role permissions.</strong> Owner / Admin / Operator /
          Viewer roles map to a capability allow-list for every sensitive action.
        </li>
        <li>
          <strong>Encryption-at-rest.</strong> Local SKYCOT installs ship with an
          encrypted-at-rest store. Hosted plans use AWS KMS-managed keys.
        </li>
        <li>
          <strong>Magic-link auth.</strong> No passwords. Short-lived bearer tokens.
          Cookie revocation table for instant force-logout.
        </li>
        <li>
          <strong>SOC 2-ready audit trail.</strong> The hash-chained log is the
          load-bearing evidence for SOC 2 controls. External SOC 2 Type II
          attestation is in progress.
        </li>
      </ul>

      <h2 style={{ fontSize: 22, fontWeight: 700, margin: "32px 0 14px" }}>
        Responsible disclosure
      </h2>
      <p style={{ fontSize: 15, lineHeight: 1.55, color: "var(--muted-foreground)" }}>
        Found something? Email{" "}
        <a href="mailto:security@skycot.com" style={{ color: "var(--primary)" }}>
          security@skycot.com
        </a>
        . We respond within 72 hours, credit reporters by name on a public
        acknowledgements page (unless you ask us not to), and run a bounty
        program for high-impact reports starting at $500.
      </p>

      <h2 style={{ fontSize: 22, fontWeight: 700, margin: "32px 0 14px" }}>
        Status page + breach notification
      </h2>
      <p style={{ fontSize: 15, lineHeight: 1.55, color: "var(--muted-foreground)" }}>
        Live status:{" "}
        <a href="https://status.skycot.com" style={{ color: "var(--primary)" }}>
          status.skycot.com
        </a>
        . If a security incident affects your data, you&apos;ll hear from us
        within 72 hours of discovery.
      </p>

      <p style={{ marginTop: 48, fontSize: 14, color: "var(--muted-foreground)" }}>
        <Link href="/landing" style={{ color: "var(--primary)" }}>
          Back to home
        </Link>
      </p>
    </main>
    </>
  );
}
