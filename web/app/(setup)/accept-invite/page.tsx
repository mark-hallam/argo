"use client";

/**
 * `/accept-invite` — landing page for the email link emitted by
 * `POST /api/v1/workspace/members/invite`.
 *
 * Stream `web-settings-members-invite-real-wire`. The page reads the
 * `?token=...` URL parameter, posts it to
 * `/api/v1/workspace/members/accept`, and on success forwards the user
 * to the signin flow with their email pre-filled — the `user_account`
 * + `user_tenant` link rows are already created server-side, so the
 * magic-link signin will land them in the right workspace.
 */

import { Suspense, useEffect, useMemo, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { acceptInvitation } from "@/lib/api/workspace-members";
import { ApiError } from "@/lib/api/types";

type AcceptState =
  | { kind: "loading" }
  | { kind: "missing_token" }
  | { kind: "ok"; email: string; tenantId: string; role: string }
  | { kind: "error"; message: string };

export default function AcceptInvitePage() {
  return (
    <Suspense fallback={<AcceptInviteShell state={{ kind: "loading" }} onSignin={() => undefined} />}>
      <AcceptInviteInner />
    </Suspense>
  );
}

function AcceptInviteInner() {
  const router = useRouter();
  const params = useSearchParams();
  const token = params?.get("token") ?? "";
  const [state, setState] = useState<AcceptState>(token ? { kind: "loading" } : { kind: "missing_token" });

  useEffect(() => {
    if (!token) return;
    let cancelled = false;
    acceptInvitation(token)
      .then((res) => {
        if (cancelled) return;
        setState({
          kind: "ok",
          email: res.email,
          tenantId: res.tenant_id,
          role: typeof res.role === "string" ? res.role : "user",
        });
        const redirect = `/signin?email=${encodeURIComponent(res.email)}&accepted_invite=1`;
        const timer = window.setTimeout(() => router.push(redirect), 1600);
        return () => window.clearTimeout(timer);
      })
      .catch((err) => {
        if (cancelled) return;
        const message =
          err instanceof ApiError ? err.message : err instanceof Error ? err.message : "Couldn't accept invitation";
        setState({ kind: "error", message });
      });
    return () => {
      cancelled = true;
    };
  }, [token, router]);

  const onSignin = useMemo(
    () => () => {
      if (state.kind === "ok") {
        router.push(`/signin?email=${encodeURIComponent(state.email)}&accepted_invite=1`);
      } else {
        router.push("/signin");
      }
    },
    [router, state],
  );

  return <AcceptInviteShell state={state} onSignin={onSignin} />;
}

function headingForState(state: AcceptState): string {
  switch (state.kind) {
    case "loading":
      return "Accepting your invitation…";
    case "missing_token":
      return "Invitation link incomplete";
    case "ok":
      return "You’re in.";
    case "error":
      return "We couldn’t accept this invitation";
  }
}

function AcceptInviteShell({
  state,
  onSignin,
}: {
  state: AcceptState;
  onSignin: () => void;
}) {
  return (
    <main
      data-testid="accept-invite"
      style={{
        maxWidth: 480,
        margin: "0 auto",
        padding: "96px 24px 48px 24px",
        fontFamily: "system-ui, -apple-system, sans-serif",
        textAlign: "center",
        color: "#14141C",
      }}
    >
      <h1 style={{ fontSize: 28, fontWeight: 600, margin: "0 0 8px 0" }}>
        {headingForState(state)}
      </h1>
      {state.kind === "loading" ? (
        <p style={{ color: "#6B6B7B", margin: 0 }}>One moment.</p>
      ) : null}
      {state.kind === "missing_token" ? (
        <>
          <p style={{ color: "#6B6B7B", margin: "0 0 24px 0" }}>
            The accept-invite URL is missing its <code>token</code> parameter. Re-open the email link or ask the
            inviter to resend.
          </p>
          <button
            type="button"
            onClick={onSignin}
            style={{
              padding: "12px 24px",
              borderRadius: 6,
              border: "1px solid #14141C",
              background: "transparent",
              cursor: "pointer",
              fontSize: 15,
            }}
          >
            Go to signin
          </button>
        </>
      ) : null}
      {state.kind === "ok" ? (
        <>
          <p style={{ color: "#6B6B7B", margin: "0 0 24px 0" }}>
            Redirecting you to signin to finish setting up your account…
          </p>
          <button
            type="button"
            onClick={onSignin}
            style={{
              padding: "12px 24px",
              borderRadius: 6,
              border: "none",
              background: "#4F46E5",
              color: "#FFFFFF",
              cursor: "pointer",
              fontSize: 15,
              fontWeight: 600,
            }}
          >
            Continue to signin
          </button>
        </>
      ) : null}
      {state.kind === "error" ? (
        <>
          <p style={{ color: "#6B6B7B", margin: "0 0 24px 0" }}>{state.message}</p>
          <button
            type="button"
            onClick={onSignin}
            style={{
              padding: "12px 24px",
              borderRadius: 6,
              border: "1px solid #14141C",
              background: "transparent",
              cursor: "pointer",
              fontSize: 15,
            }}
          >
            Go to signin
          </button>
        </>
      ) : null}
    </main>
  );
}
