/**
 * Strip internal protocol detail from onboarding error copy.
 *
 * Background (F5b from chrome-live-walk-and-playwright 2026-05-26-2007):
 * the gateway's auth middleware returns errors like
 * `X-Argo-Principal header must carry a verified human:<email> value`.
 * Before this scrubber, the workspace step's catch block rendered that
 * string verbatim inside the form card — leaking internal header names
 * to investors. The scrubber detects internal-protocol substrings and
 * substitutes user-friendly copy, while passing benign messages through
 * unchanged.
 */

const PROTOCOL_LEAK_PATTERNS: ReadonlyArray<RegExp> = [
  /x-argo-principal/i,
  /\bprincipal\b/i,
  /x-argo-/i,
  /bearer\s+token/i,
  /authorization\s+header/i,
];

// Honest substitute for an auth-reject leak. The gateway returns an
// auth-reject (the `X-Argo-Principal` leak above) when the session was
// dropped, most often because the app was opened on a non-canonical
// loopback host (`127.0.0.1` vs `localhost`), which un-pins the
// `__Host-session` cookie. Claiming the workspace was non-persistent
// (the old copy) was FALSE and misdirected the user away from the real
// fix, which is to re-authenticate. The
// middleware now canonicalizes the host so the session survives; if a
// reject still reaches here, the honest message is that the session
// expired and they should sign in again.
const SESSION_EXPIRED_COPY =
  "Your session expired. Please sign in again to keep your changes.";
const GENERIC_FAILURE_COPY = "Something went wrong. Please try again.";

export interface ScrubbedError {
  /** Safe-to-render string. Never contains internal protocol names. */
  message: string;
  /** True if the original error mentioned an internal protocol detail. */
  wasProtocolLeak: boolean;
}

export function scrubOnboardingErrorMessage(input: unknown): ScrubbedError {
  const raw =
    input instanceof Error
      ? input.message
      : typeof input === "string"
        ? input
        : "";
  if (raw.length === 0) {
    return { message: GENERIC_FAILURE_COPY, wasProtocolLeak: false };
  }
  const isLeak = PROTOCOL_LEAK_PATTERNS.some((re) => re.test(raw));
  if (isLeak) {
    return { message: SESSION_EXPIRED_COPY, wasProtocolLeak: true };
  }
  return { message: raw, wasProtocolLeak: false };
}
