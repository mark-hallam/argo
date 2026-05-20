"use client";

/**
 * Loading placeholder for the `/hire` wizard steps (replaces the
 * previous `Suspense fallback={null}` which left users on a white
 * screen during draft hydration).
 *
 * Shape-matches the `WizardEditorialShell`: A-style top bar (brand
 * mark + step pips + Help / SAVE & EXIT chips) plus a centred
 * editorial stage with placeholders for the eyebrow pill, Audiowide
 * hero, sub paragraph, body, and CTA row. Pulses via Tailwind
 * `animate-pulse`. The skeleton runs in a server-friendly markup
 * with inline styles — no global CSS, no fonts to wait on.
 */
export interface WizardSkeletonProps {
  /** Which step is being hydrated (1..6). Drives the pip strip. */
  step?: 1 | 2 | 3 | 4 | 5 | 6;
  className?: string;
  "data-testid"?: string;
}

const TOTAL_STEPS = 5;

export function WizardSkeleton({
  step = 1,
  className,
  "data-testid": testId,
}: WizardSkeletonProps) {
  const filledUpTo = step === 6 ? TOTAL_STEPS : step;
  return (
    <div
      data-testid={testId ?? "wizard-skeleton"}
      aria-busy="true"
      aria-live="polite"
      aria-label="Loading hire wizard"
      className={className}
      style={{
        minHeight: "100vh",
        background: "#fafaff",
        position: "relative",
        fontFamily: "Inter, sans-serif",
      }}
    >
      <header
        style={{
          padding: "22px 48px",
          display: "grid",
          gridTemplateColumns: "1fr auto 1fr",
          alignItems: "center",
          borderBottom: "1px solid #ece9f5",
          background: "rgba(255,255,255,0.7)",
          backdropFilter: "blur(12px)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <div
            className="animate-pulse"
            style={{
              width: 28,
              height: 28,
              borderRadius: 7,
              background: "linear-gradient(135deg, #8b5cf6, #ec4899)",
              opacity: 0.6,
            }}
          />
          <div
            className="animate-pulse"
            style={{ width: 80, height: 14, borderRadius: 4, background: "#ece9f5" }}
          />
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <div style={{ width: 30, height: 10, borderRadius: 3, background: "#ece9f5" }} className="animate-pulse" />
          <div style={{ display: "flex", gap: 4, margin: "0 12px" }}>
            {Array.from({ length: TOTAL_STEPS }).map((_, idx) => {
              const filled = idx + 1 <= filledUpTo;
              return (
                <span
                  key={idx}
                  data-testid={`wizard-skeleton-pip-${idx + 1}`}
                  style={{
                    width: 24,
                    height: 3,
                    borderRadius: 2,
                    background: filled
                      ? "linear-gradient(135deg, #8b5cf6, #ec4899)"
                      : "#f4f4f4",
                  }}
                />
              );
            })}
          </div>
          <div style={{ width: 56, height: 10, borderRadius: 3, background: "#ece9f5" }} className="animate-pulse" />
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 16, justifyContent: "flex-end" }}>
          <div style={{ width: 50, height: 28, borderRadius: 7, background: "#ece9f5" }} className="animate-pulse" />
          <div style={{ width: 80, height: 14, borderRadius: 4, background: "#ece9f5" }} className="animate-pulse" />
        </div>
      </header>
      <main
        style={{
          maxWidth: 840,
          margin: "0 auto",
          padding: "96px 48px 80px",
          textAlign: "center",
        }}
      >
        <div
          className="animate-pulse"
          style={{
            display: "inline-block",
            width: 140,
            height: 24,
            borderRadius: 999,
            background: "#ece9f5",
            marginBottom: 28,
          }}
        />
        <div
          className="animate-pulse"
          style={{
            margin: "0 auto 16px",
            width: "min(600px, 80%)",
            height: 64,
            borderRadius: 10,
            background: "#ece9f5",
          }}
        />
        <div
          className="animate-pulse"
          style={{
            margin: "0 auto 48px",
            width: "min(420px, 60%)",
            height: 18,
            borderRadius: 5,
            background: "#ece9f5",
          }}
        />
        <div
          className="animate-pulse"
          style={{
            margin: "0 auto 40px",
            maxWidth: 640,
            height: 160,
            borderRadius: 14,
            background: "white",
            border: "1px solid #ece9f5",
          }}
        />
        <div style={{ display: "flex", gap: 16, justifyContent: "center", flexWrap: "wrap" }}>
          <div
            className="animate-pulse"
            style={{ width: 110, height: 56, borderRadius: 14, background: "#ece9f5" }}
          />
          <div
            className="animate-pulse"
            style={{
              width: 180,
              height: 56,
              borderRadius: 14,
              background: "linear-gradient(135deg, #d8c7f7, #f5c5d8)",
            }}
          />
        </div>
      </main>
    </div>
  );
}
