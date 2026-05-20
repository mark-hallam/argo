/**
 * Resolve target URLs + run mode for the onboarding E2E.
 *
 * Modes:
 * - `local` / `ci` (default): three Next.js servers spun up by the test
 *   harness; backend calls are intercepted via `page.route()` so no
 *   Rust gateway is required.
 * - `deploy-smoke`: target the live deployment; backend calls hit the
 *   real gateway and the magic link is fetched from a real test inbox.
 *
 * Selection: `ARGO_E2E_TARGET=deploy` → deploy-smoke; otherwise local.
 */

export type RunMode = "local" | "deploy-smoke";

export interface E2eEnv {
  mode: RunMode;
  marketingUrl: string;
  spaUrl: string;
  gatewayUrl: string;
  tenantProvisionReady: boolean;
  magicLinkTimeoutMs: number;
}

export function resolveEnv(): E2eEnv {
  const mode: RunMode = process.env.ARGO_E2E_TARGET === "deploy" ? "deploy-smoke" : "local";
  return {
    mode,
    marketingUrl: process.env.E2E_MARKETING_URL ?? (mode === "deploy-smoke" ? "https://skycot.com" : "http://localhost:3001"),
    spaUrl: process.env.E2E_SPA_URL ?? (mode === "deploy-smoke" ? "https://app.skycot.com" : "http://localhost:3000"),
    gatewayUrl: process.env.E2E_GATEWAY_URL ?? (mode === "deploy-smoke" ? "https://api.skycot.com" : "http://127.0.0.1:7400"),
    tenantProvisionReady: process.env.E2E_TENANT_PROVISION_READY === "1",
    magicLinkTimeoutMs: Number(process.env.E2E_MAGIC_LINK_TIMEOUT_MS ?? 60_000),
  };
}

export function uniqueTestEmail(): string {
  const stamp = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
  return `e2e-test-${stamp}@throwaway.local`;
}
