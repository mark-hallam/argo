/**
 * Run-now failure → user-facing toast/chip copy.
 *
 * Maps the gateway's `ApiError` envelope (or any thrown `Error`) into a
 * (headline, description) pair surfaced via sonner. The chip in the
 * run-history strip uses the same headline so the persistent state and
 * the transient toast tell the same story.
 *
 * Closes finding F08 from `whole-site-mvp-claims-coverage.md`: prior
 * behaviour was a silent `toast.error("Run failed: <raw msg>")` with
 * no 403 special-case and no persistent indicator.
 */
import { ApiError } from "@/lib/api/types";

export interface RunNowErrorCopy {
  headline: string;
  description: string;
}

export function describeRunNowError(err: unknown): RunNowErrorCopy {
  if (err instanceof ApiError) {
    if (err.status === 403) {
      return {
        headline: "Run blocked — workflow execute not permitted",
        description:
          "Demo principal cannot execute workflows yet. Switch to a real account or contact your workspace admin.",
      };
    }
    const codePrefix = err.code ? `${err.code} · ` : "";
    return {
      headline: `Run failed — HTTP ${err.status}`,
      description: `${codePrefix}${err.message || "see audit log"}`,
    };
  }
  const message = err instanceof Error && err.message ? err.message : "Network error";
  return {
    headline: "Run failed — see audit log",
    description: message,
  };
}
