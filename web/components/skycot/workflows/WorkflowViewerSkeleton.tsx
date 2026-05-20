"use client";

/**
 * Loading placeholder for the `/workflows/<v>/<name>` viewer page.
 *
 * Shape-matches the live timeline so the page doesn't shift on
 * hydrate: header card (vertical tag + risk + version + name +
 * description + action buttons) + 5 timeline step placeholders
 * connected by the brand gradient rail.
 */
export interface WorkflowViewerSkeletonProps {
  /** Number of step placeholders to render. Defaults to 5. */
  steps?: number;
  className?: string;
  "data-testid"?: string;
}

export function WorkflowViewerSkeleton({
  steps = 5,
  className,
  "data-testid": testId,
}: WorkflowViewerSkeletonProps) {
  return (
    <div
      data-testid={testId ?? "workflow-viewer-skeleton"}
      aria-busy="true"
      aria-live="polite"
      aria-label="Loading workflow"
      className={className}
      style={{ padding: "32px 32px 96px", maxWidth: 900, margin: "0 auto" }}
    >
      <div style={{ width: 140, height: 11, borderRadius: 3, background: "#ece9f5", marginBottom: 24 }} className="animate-pulse" />

      <div
        style={{
          background: "white",
          border: "1px solid #ece9f5",
          borderRadius: 16,
          padding: 28,
          marginBottom: 24,
          position: "relative",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            position: "absolute",
            top: 0,
            left: 0,
            width: 4,
            height: "100%",
            background: "linear-gradient(180deg, #8b5cf6, #ec4899, #ff6b6b)",
            opacity: 0.45,
          }}
        />
        <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 14 }}>
          <div style={{ width: 80, height: 11, borderRadius: 3, background: "#ece9f5" }} className="animate-pulse" />
          <div style={{ width: 110, height: 16, borderRadius: 999, background: "#ece9f5" }} className="animate-pulse" />
          <div style={{ marginLeft: "auto", width: 80, height: 12, borderRadius: 3, background: "#ece9f5" }} className="animate-pulse" />
        </div>
        <div style={{ width: 240, height: 28, borderRadius: 6, background: "#ece9f5", marginBottom: 14 }} className="animate-pulse" />
        <div style={{ width: "85%", height: 12, borderRadius: 4, background: "#ece9f5", marginBottom: 8 }} className="animate-pulse" />
        <div style={{ width: "70%", height: 12, borderRadius: 4, background: "#ece9f5", marginBottom: 18 }} className="animate-pulse" />
        <div style={{ display: "flex", gap: 10 }}>
          <div style={{ width: 132, height: 34, borderRadius: 9, background: "linear-gradient(135deg, #d8c7f7, #f5c5d8)" }} className="animate-pulse" />
          <div style={{ width: 130, height: 34, borderRadius: 9, background: "#ece9f5" }} className="animate-pulse" />
          <div style={{ width: 100, height: 34, borderRadius: 9, background: "#ece9f5" }} className="animate-pulse" />
        </div>
      </div>

      <div style={{ width: 110, height: 11, borderRadius: 3, background: "#ece9f5", margin: "32px 0 14px" }} className="animate-pulse" />

      <div style={{ position: "relative" }}>
        <div
          style={{
            position: "absolute",
            left: 18,
            top: 22,
            bottom: 22,
            width: 2,
            background: "linear-gradient(180deg, #8b5cf6, #ec4899, #ff6b6b)",
            opacity: 0.35,
          }}
        />
        {Array.from({ length: steps }, (_, idx) => (
          <div
            key={idx}
            data-testid={`workflow-viewer-skeleton-step-${idx}`}
            style={{ position: "relative", padding: "0 0 18px 56px", minHeight: 64 }}
          >
            <div
              className="animate-pulse"
              style={{
                position: "absolute",
                left: 0,
                top: 0,
                width: 38,
                height: 38,
                borderRadius: 10,
                background: "#ece9f5",
                boxShadow: "0 4px 10px -3px rgba(0,0,0,0.08)",
              }}
            />
            <div
              className="animate-pulse"
              style={{
                background: "white",
                border: "1px solid #ece9f5",
                borderRadius: 12,
                padding: "16px 18px",
              }}
            >
              <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 6 }}>
                <div style={{ width: 96, height: 14, borderRadius: 999, background: "#ece9f5" }} />
                <div style={{ width: 180, height: 14, borderRadius: 4, background: "#ece9f5" }} />
              </div>
              <div style={{ width: "84%", height: 10, borderRadius: 3, background: "#ece9f5", marginBottom: 6 }} />
              <div style={{ width: "62%", height: 10, borderRadius: 3, background: "#ece9f5" }} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
