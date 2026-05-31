"use client";

import { useState } from "react";
import { signupSchema, type SignupInput } from "./signup-schema";

export const SIGNUP_REQUEST_TIMEOUT_MS = 12_000;

export type SignupState =
  | { kind: "idle" }
  | { kind: "submitting" }
  | { kind: "sent"; email: string }
  | { kind: "error"; code: SignupErrorCode; message: string };

export type SignupErrorCode =
  | "invalid_email"
  | "rate_limited"
  | "timeout"
  | "network"
  | "server";

interface SignupResponseBody {
  status?: "sent";
  error?: { code: string; message: string };
}

class SignupRequestTimeoutError extends Error {
  constructor() {
    super("Signup request timed out");
    this.name = "SignupRequestTimeoutError";
  }
}

function isAbortError(error: unknown): boolean {
  return (
    typeof DOMException !== "undefined" &&
    error instanceof DOMException &&
    error.name === "AbortError"
  );
}

function isSignupTimeout(error: unknown): boolean {
  return error instanceof SignupRequestTimeoutError || isAbortError(error);
}

function fetchWithClientTimeout(
  input: RequestInfo | URL,
  init: RequestInit,
  timeoutMs = SIGNUP_REQUEST_TIMEOUT_MS,
): Promise<Response> {
  const controller = new AbortController();

  return new Promise<Response>((resolve, reject) => {
    const timeoutId = setTimeout(() => {
      reject(new SignupRequestTimeoutError());
      controller.abort();
    }, timeoutMs);

    fetch(input, { ...init, signal: controller.signal }).then(
      (res) => {
        clearTimeout(timeoutId);
        resolve(res);
      },
      (error: unknown) => {
        clearTimeout(timeoutId);
        reject(error);
      },
    );
  });
}

export function useSignup() {
  const [state, setState] = useState<SignupState>({ kind: "idle" });

  async function submit(input: SignupInput) {
    const parsed = signupSchema.safeParse(input);
    if (!parsed.success) {
      const message = parsed.error.issues[0]?.message ?? "Invalid email";
      setState({ kind: "error", code: "invalid_email", message });
      return { ok: false as const, code: "invalid_email" as const };
    }
    setState({ kind: "submitting" });
    try {
      const res = await fetchWithClientTimeout("/api/auth/signup", {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ email: parsed.data.email }),
      });
      const body = (await res.json().catch(() => ({}))) as SignupResponseBody;
      if (res.status === 429) {
        const message =
          body.error?.message ??
          "Too many attempts. Wait an hour and try again.";
        setState({ kind: "error", code: "rate_limited", message });
        return { ok: false as const, code: "rate_limited" as const };
      }
      if (!res.ok) {
        const message = body.error?.message ?? "Couldn't send magic link.";
        setState({ kind: "error", code: "server", message });
        return { ok: false as const, code: "server" as const };
      }
      setState({ kind: "sent", email: parsed.data.email });
      return { ok: true as const };
    } catch (error) {
      if (isSignupTimeout(error)) {
        setState({
          kind: "error",
          code: "timeout",
          message:
            "Signup took too long to respond. Check your connection and try again.",
        });
        return { ok: false as const, code: "timeout" as const };
      }
      setState({
        kind: "error",
        code: "network",
        message: "Couldn't reach SKYCOT. Check your connection and try again.",
      });
      return { ok: false as const, code: "network" as const };
    }
  }

  function reset() {
    setState({ kind: "idle" });
  }

  return { state, submit, reset } as const;
}
