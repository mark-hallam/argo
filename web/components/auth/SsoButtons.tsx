"use client";

import { startOAuth, type OAuthProvider } from "@/lib/api/sso";

/**
 * Active SSO button row — replaces the v1.x placeholder buttons that
 * shipped with the editorial-A auth design lock (2026-05-18). Clicking
 * a button navigates to the gateway's OAuth start endpoint, which
 * redirects to the provider, then back to `/auth/oauth/<provider>/callback`.
 *
 * `highlight` lets the signin page emphasize the provider matched by
 * SSO-discovery (e.g. "Continue with Google" border-glows when the
 * user's workspace OAuth-only-binds to Google).
 *
 * `redirectTo` is forwarded to the OAuth start endpoint so the
 * gateway can route the user back to their original destination after
 * sign-in.
 */
export function SsoButtons({
  redirectTo,
  highlight,
  disabled,
}: {
  redirectTo?: string;
  highlight?: OAuthProvider;
  disabled?: boolean;
}) {
  return (
    <div className="sso-row" data-testid="sso-row">
      {PROVIDERS.map((p) => (
        <SsoButton
          key={p.id}
          provider={p}
          highlighted={highlight === p.id}
          redirectTo={redirectTo}
          disabled={disabled}
        />
      ))}
    </div>
  );
}

interface ProviderMeta {
  id: OAuthProvider;
  label: string;
  /** Inline SVG logo — kept small to avoid an extra network round-trip. */
  logo: React.ReactNode;
}

const PROVIDERS: ProviderMeta[] = [
  {
    id: "google",
    label: "Continue with Google",
    logo: <GoogleMark />,
  },
  {
    id: "github",
    label: "Continue with GitHub",
    logo: <GitHubMark />,
  },
  {
    id: "microsoft",
    label: "Continue with Microsoft",
    logo: <MicrosoftMark />,
  },
];

function SsoButton({
  provider,
  highlighted,
  redirectTo,
  disabled,
}: {
  provider: ProviderMeta;
  highlighted: boolean;
  redirectTo?: string;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      className={
        highlighted ? "sso-btn sso-btn-active sso-btn-highlight" : "sso-btn sso-btn-active"
      }
      onClick={() => startOAuth(provider.id, redirectTo)}
      disabled={disabled}
      aria-disabled={disabled || undefined}
      data-testid={`sso-button-${provider.id}`}
      data-provider={provider.id}
    >
      <span className="logo" aria-hidden>
        {provider.logo}
      </span>
      {provider.label}
    </button>
  );
}

function GoogleMark() {
  return (
    <svg viewBox="0 0 18 18" width="18" height="18" aria-hidden focusable="false">
      <path
        fill="#4285F4"
        d="M17.64 9.2c0-.64-.06-1.25-.17-1.84H9v3.48h4.84a4.14 4.14 0 0 1-1.79 2.71v2.26h2.9c1.7-1.56 2.69-3.87 2.69-6.61z"
      />
      <path
        fill="#34A853"
        d="M9 18c2.43 0 4.47-.8 5.96-2.19l-2.9-2.26c-.8.54-1.84.86-3.06.86-2.35 0-4.34-1.58-5.05-3.71H.96v2.33A9 9 0 0 0 9 18z"
      />
      <path
        fill="#FBBC05"
        d="M3.95 10.7A5.41 5.41 0 0 1 3.66 9c0-.59.1-1.16.29-1.7V4.96H.96A9 9 0 0 0 0 9c0 1.45.35 2.83.96 4.04l2.99-2.34z"
      />
      <path
        fill="#EA4335"
        d="M9 3.58c1.32 0 2.51.45 3.44 1.34l2.58-2.58C13.46.89 11.43 0 9 0A9 9 0 0 0 .96 4.96l2.99 2.33C4.66 5.16 6.65 3.58 9 3.58z"
      />
    </svg>
  );
}

function GitHubMark() {
  return (
    <svg viewBox="0 0 16 16" width="18" height="18" aria-hidden focusable="false">
      <path
        style={{ fill: "var(--foreground)" }}
        d="M8 0a8 8 0 0 0-2.53 15.59c.4.07.55-.17.55-.38v-1.33c-2.22.48-2.69-1.07-2.69-1.07-.36-.92-.89-1.17-.89-1.17-.73-.5.06-.49.06-.49.8.06 1.23.83 1.23.83.72 1.23 1.88.87 2.34.66.07-.52.28-.87.51-1.07-1.77-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.83-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.66 7.66 0 0 1 4 0c1.53-1.03 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.52.56.82 1.28.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.74.54 1.49v2.21c0 .21.15.46.55.38A8 8 0 0 0 8 0z"
      />
    </svg>
  );
}

function MicrosoftMark() {
  return (
    <svg viewBox="0 0 18 18" width="18" height="18" aria-hidden focusable="false">
      <path fill="#F25022" d="M0 0h8.55v8.55H0z" />
      <path fill="#7FBA00" d="M9.45 0H18v8.55H9.45z" />
      <path fill="#00A4EF" d="M0 9.45h8.55V18H0z" />
      <path fill="#FFB900" d="M9.45 9.45H18V18H9.45z" />
    </svg>
  );
}
