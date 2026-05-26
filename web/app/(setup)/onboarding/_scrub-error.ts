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

const DEMO_MODE_LEAK_COPY =
  "We can't save this in demo mode. Sign in to continue with a real workspace.";
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
    return { message: DEMO_MODE_LEAK_COPY, wasProtocolLeak: true };
  }
  return { message: raw, wasProtocolLeak: false };
}
