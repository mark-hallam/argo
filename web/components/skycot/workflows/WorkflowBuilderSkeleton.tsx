"use client";

/**
 * Loading placeholder for the `/workflows/<v>/<name>/build` canvas.
 *
 * Shape-matches the live builder so the layout doesn't shift on
 * hydrate: top bar (back link + title + actions), AI prompt bar,
 * canvas grid with three card-shaped node placeholders, and a
 * 360px side panel. Pulses via Tailwind `animate-pulse`.
 */
export interface WorkflowBuilderSkeletonProps {
  className?: string;
  "data-testid"?: string;
}

export function WorkflowBuilderSkeleton({
  className,
  "data-testid": testId,
}: WorkflowBuilderSkeletonProps) {
  return (
    <div
      data-testid={testId ?? "workflow-builder-skeleton"}
      aria-busy="true"
      aria-live="polite"
      aria-label="Loading workflow builder"
      className={className}
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        background: "var(--background)",
      }}
    >
      <div
        style={{
          padding: "14px 28px",
          background: "white",
          borderBottom: "1px solid #ece9f5",
          display: "flex",
          alignItems: "center",
          gap: 16,
        }}
      >
        <div style={{ width: 120, height: 12, borderRadius: 4, background: "#ece9f5" }} className="animate-pulse" />
        <div style={{ width: 90, height: 12, borderRadius: 4, background: "#ece9f5" }} className="animate-pulse" />
        <div style={{ width: 160, height: 18, borderRadius: 6, background: "#ece9f5" }} className="animate-pulse" />
        <div style={{ marginLeft: "auto", display: "flex", gap: 10 }}>
          <div style={{ width: 90, height: 32, borderRadius: 9, background: "#ece9f5" }} className="animate-pulse" />
          <div style={{ width: 110, height: 32, borderRadius: 9, background: "#ece9f5" }} className="animate-pulse" />
          <div style={{ width: 130, height: 32, borderRadius: 9, background: "linear-gradient(135deg, #d8c7f7, #f5c5d8)" }} className="animate-pulse" />
        </div>
      </div>

      <div
        style={{
          padding: "12px 28px",
          background: "linear-gradient(180deg, rgba(139,92,246,0.04), transparent)",
          borderBottom: "1px solid #ece9f5",
          display: "flex",
          gap: 10,
          alignItems: "center",
        }}
      >
        <div style={{ width: 32, height: 32, borderRadius: 9, background: "linear-gradient(135deg, var(--primary), var(--gradient-end))", opacity: 0.5 }} className="animate-pulse" />
        <div style={{ flex: 1, height: 38, borderRadius: 9, background: "white", border: "1px solid #ece9f5" }} className="animate-pulse" />
        <div style={{ width: 100, height: 38, borderRadius: 9, background: "linear-gradient(135deg, #d8c7f7, #f5c5d8)" }} className="animate-pulse" />
      </div>

      <div style={{ flex: 1, display: "flex", minHeight: 0, position: "relative" }}>
        <div
          style={{
            flex: 1,
            backgroundImage:
              "radial-gradient(circle, var(--border) 1px, transparent 1px)",
            backgroundSize: "20px 20px",
            position: "relative",
            padding: 60,
          }}
        >
          <div style={{ display: "flex", flexDirection: "column", gap: 32, alignItems: "center" }}>
            {[0, 1, 2].map((idx) => (
              <div
                key={idx}
                data-testid={`workflow-builder-skeleton-node-${idx}`}
                className="animate-pulse"
                style={{
                  width: 280,
                  background: "white",
                  border: "2px solid rgba(0,0,0,0.06)",
                  borderRadius: 14,
                  boxShadow: "0 6px 16px -8px rgba(0,0,0,0.12)",
                  overflow: "hidden",
                }}
              >
                <div
                  style={{
                    padding: "12px 16px",
                    background: "linear-gradient(135deg, rgba(139,92,246,0.1), rgba(139,92,246,0.04))",
                    borderBottom: "1px solid rgba(139,92,246,0.18)",
                    display: "flex",
                    alignItems: "center",
                    gap: 10,
                  }}
                >
                  <div style={{ width: 28, height: 28, borderRadius: 7, background: "#ece9f5" }} />
                  <div style={{ flex: 1 }}>
                    <div style={{ width: 64, height: 8, borderRadius: 3, background: "#ece9f5", marginBottom: 6 }} />
                    <div style={{ width: 140, height: 12, borderRadius: 4, background: "#ece9f5" }} />
                  </div>
                </div>
                <div style={{ padding: "12px 16px" }}>
                  <div style={{ width: "92%", height: 9, borderRadius: 3, background: "#ece9f5", marginBottom: 6 }} />
                  <div style={{ width: "75%", height: 9, borderRadius: 3, background: "#ece9f5" }} />
                </div>
              </div>
            ))}
          </div>
        </div>
        <aside
          style={{
            width: 360,
            background: "white",
            borderLeft: "1px solid #ece9f5",
            padding: 24,
          }}
        >
          <div style={{ width: 70, height: 10, borderRadius: 3, background: "#ece9f5", marginBottom: 14 }} className="animate-pulse" />
          <div style={{ width: 180, height: 20, borderRadius: 5, background: "#ece9f5", marginBottom: 18 }} className="animate-pulse" />
          <div style={{ width: "100%", height: 9, borderRadius: 3, background: "#ece9f5", marginBottom: 6 }} className="animate-pulse" />
          <div style={{ width: "92%", height: 9, borderRadius: 3, background: "#ece9f5", marginBottom: 6 }} className="animate-pulse" />
          <div style={{ width: "78%", height: 9, borderRadius: 3, background: "#ece9f5", marginBottom: 24 }} className="animate-pulse" />
          <div style={{ width: 80, height: 9, borderRadius: 3, background: "#ece9f5", marginBottom: 10 }} className="animate-pulse" />
          <div style={{ width: "100%", height: 36, borderRadius: 9, background: "#ece9f5" }} className="animate-pulse" />
        </aside>
      </div>
    </div>
  );
}
