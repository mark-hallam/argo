import Link from "next/link";

/**
 * Help — v1.0 placeholder. Wave-15 every-button audit flagged
 * /help as a 404 reachable from the WizardEditorialShell + auth
 * shells' top-right "Help" link (visible on every onboarding +
 * /hire + /auth step). Stub page until a richer help center
 * lands; routes the user back to where they can actually get
 * answers.
 */
export default function HelpPage() {
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
        Help · v1.0
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
        Need a hand?
      </h1>
      <p style={{ fontSize: 18, lineHeight: 1.5, margin: "0 0 32px", color: "#52525b" }}>
        We&apos;re building the in-product help center alongside v1.1. In the
        meantime — the three best places to get answers fast:
      </p>

      <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "grid", gap: 18 }}>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>
            📚 Docs
          </strong>
          <Link href="/docs" style={{ color: "#8b5cf6", fontWeight: 600 }}>
            skycot.com/docs
          </Link>{" "}
          — getting-started guides, vertical templates, workflow recipes, API
          reference (the bits that are written so far).
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>
            💬 Email
          </strong>
          <a href="mailto:hello@skycot.com" style={{ color: "#8b5cf6", fontWeight: 600 }}>
            hello@skycot.com
          </a>{" "}
          — Mark (founder) reads every inbound. Reply usually within a few
          business hours.
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>
            🔍 Status
          </strong>
          <a
            href="https://status.skycot.com"
            style={{ color: "#8b5cf6", fontWeight: 600 }}
          >
            status.skycot.com
          </a>{" "}
          — uptime + incident history.
        </li>
        <li>
          <strong style={{ display: "block", marginBottom: 4 }}>
            🛡️ Security
          </strong>
          <Link href="/security" style={{ color: "#8b5cf6", fontWeight: 600 }}>
            Responsible disclosure inbox + posture
          </Link>
        </li>
      </ul>

      <p style={{ marginTop: 48, fontSize: 14, color: "#71717a" }}>
        Back to{" "}
        <Link href="/landing" style={{ color: "#8b5cf6" }}>
          home
        </Link>
        .
      </p>
    </main>
  );
}
