import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Hire — SKYCOT",
};

/**
 * Route-group layout for the chrome-free hire flow.
 *
 * Each step page inside `(hire)/` owns its own `HireShell` instance
 * because step number, validation state, and footer-nav handlers are
 * page-specific. The layout is intentionally pass-through so the
 * root `<body>` providers (theme, query, auth) still wrap every
 * step.
 *
 * If we later need shared client-side state across the whole hire
 * flow (e.g. a draft context provider that all steps consume), it
 * goes here as a "use client" wrapper around `{children}`. For v1
 * the `useHireDraft()` hook hydrates from localStorage at the page
 * level, which is sufficient.
 */
export default function HireGroupLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
