"use client";

import { useEffect } from "react";

/**
 * Outermost crash boundary — Next.js renders this when even the root
 * layout fails (e.g. one of the 5 providers in `app/layout.tsx` throws
 * during hydration). It MUST render its own `<html>` + `<body>` since
 * the root layout is the thing that broke.
 *
 * Deliberately minimal: inline styles only (Tailwind may not be
 * available), no theme provider (it failed), no router (may also have
 * failed). The page reload button is the only way out.
 */
export default function GlobalError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot] global error:", error);
    }
  }, [error]);

  const bg = "linear-gradient(135deg, #fafaff 0%, #f4f0fa 100%)";
  const ink = "#1e1b4b";
  const muted = "#52525b";
  const gradient = "linear-gradient(135deg, #8b5cf6, #ec4899)";

  return (
    <html lang="en">
      <body
        style={{
          margin: 0,
          minHeight: "100vh",
          fontFamily:
            'Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif',
          background: bg,
          color: ink,
          display: "grid",
          placeItems: "center",
          padding: "24px",
        }}
      >
        <main
          style={{
            maxWidth: 520,
            textAlign: "center",
            background: "white",
            border: "1px solid #ece9f5",
            borderRadius: 16,
            padding: "40px 32px",
            boxShadow: "0 10px 30px rgba(30, 27, 75, 0.08)",
          }}
        >
          <p
            style={{
              fontFamily: '"JetBrains Mono", ui-monospace, monospace',
              fontSize: 11,
              letterSpacing: "0.2em",
              textTransform: "uppercase",
              color: muted,
              margin: "0 0 14px",
            }}
          >
            500 · Unrecoverable
          </p>
          <h1
            style={{
              fontFamily: 'Audiowide, Inter, sans-serif',
              fontSize: 32,
              fontWeight: 400,
              lineHeight: 1.1,
              margin: "0 0 12px",
              backgroundImage: gradient,
              backgroundClip: "text",
              WebkitBackgroundClip: "text",
              color: "transparent",
              WebkitTextFillColor: "transparent",
            }}
          >
            We hit a wall.
          </h1>
          <p style={{ color: muted, fontSize: 15, lineHeight: 1.55, margin: "0 0 24px" }}>
            Something broke deep enough that the page couldn&apos;t recover.
            Reloading usually fixes it. We&apos;ve been notified either way.
          </p>
          <button
            type="button"
            onClick={reset}
            style={{
              backgroundImage: gradient,
              color: "white",
              border: 0,
              padding: "12px 24px",
              borderRadius: 999,
              fontSize: 14,
              fontWeight: 600,
              cursor: "pointer",
            }}
          >
            Reload the page
          </button>
          <p style={{ color: muted, fontSize: 12, marginTop: 24 }}>
            Still stuck?{" "}
            <a
              href="mailto:support@skycot.com"
              style={{ color: "#8b5cf6", textDecoration: "none" }}
            >
              support@skycot.com
            </a>
          </p>
        </main>
      </body>
    </html>
  );
}
