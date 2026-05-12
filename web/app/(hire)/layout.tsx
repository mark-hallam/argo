import type { ReactNode } from "react";

/**
 * Route-group layout for the chrome-free hire flow. Each step page
 * inside `(hire)/` owns its own `HireShell` instance because step
 * count + progress is page-driven. The layout is intentionally
 * pass-through so the root `<body>` providers (theme, query,
 * auth) still wrap every step.
 */
export default function HireGroupLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
