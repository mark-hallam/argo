import Link from "next/link";

/**
 * Privacy page — v1.0 placeholder until full legal review lands.
 * The bullets reflect the actual product posture today (workspace
 * isolation, no training on customer data, encryption-at-rest,
 * audit chain). Full policy text follows the public launch.
 */
export default function PrivacyPage() {
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600&display=swap"
        rel="stylesheet"
      />
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
          fontFamily: '"JetBrains Mono", monospace',
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
        Privacy.
      </h1>
      <p style={{ fontSize: 18, lineHeight: 1.5, margin: "0 0 32px", color: "var(--muted-foreground)" }}>
        SKYCOT exists to be the safest way to put AI inside your business.
        Until our full privacy policy lands, here&apos;s what&apos;s true today.
      </p>

      <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "grid", gap: 20 }}>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>
            Workspace isolation by construction.
          </strong>
          Every audit row, skill run, integration call, and memory entry carries
          your workspace stamp. Automated checks block any code that could leak
          across the boundary.
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>
            No training on your data.
          </strong>
          We do not use your content to train any model, ours or anyone else&apos;s.
          Your data flows through approved language-model providers under your
          API keys (BYOK on Pro+).
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>
            Encryption-at-rest.
          </strong>
          Local SKYCOT installs ship with encrypted-at-rest storage. Hosted
          plans use AWS KMS-managed keys.
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>
            Hash-chained audit trail.
          </strong>
          Every action a teammate takes is recorded with a tamper-evident hash
          chain. You can prove no entry has been altered post-fact.
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>
            Data subject rights (GDPR/CCPA).
          </strong>
          Email <a href="mailto:privacy@skycot.com" style={{ color: "var(--primary)" }}>privacy@skycot.com</a> for access,
          correction, deletion, portability, or opt-out requests. We&apos;ll respond
          within 30 days.
        </li>
      </ul>

      <p style={{ marginTop: 48, fontSize: 14, color: "var(--muted-foreground)" }}>
        Full policy under legal review.{" "}
        <Link href="/landing" style={{ color: "var(--primary)" }}>
          Back to home
        </Link>
        .
      </p>
    </main>
    </>
  );
}
