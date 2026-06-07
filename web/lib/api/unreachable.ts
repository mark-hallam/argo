/**
 * Shared "the gateway gave us no data" classifier for the `lib/api/*`
 * fetchers. Consolidates ~24 byte-identical private copies into one home.
 *
 * Returns `true` for:
 *  - an `ApiError` with status 404 — the endpoint isn't live / the resource
 *    doesn't exist, which can reasonably degrade to a fallback or empty state;
 *  - a `TypeError` — the browser's signal for a CORS preflight failure, DNS
 *    error, or refused connection (the gateway is unreachable).
 *
 * Deliberately returns `false` for {@link GatewayTimeoutError}. A timeout is
 * NOT the same class of failure: a 404 / network-down means "there is no data
 * to show", but a timeout means "we couldn't load your data IN TIME" — and the
 * honest terminal state for that is an error + retry, NEVER a false-empty
 * ("you have zero X"). By leaving the timeout unclassified here, it PROPAGATES
 * out of the fetcher's catch, so React Query enters `isError` and the page
 * renders its error state. (A separate proposal to include
 * `GatewayTimeoutError` here was rejected: catching it would swallow the error
 * into a fallback and reintroduce exactly the misleading empty list this
 * helper's callers are trying to avoid.)
 *
 * Variants NOT covered by this canonical shape keep their own local copy on
 * purpose: `team.ts` (treats a timeout as an empty roster), `workers.ts`
 * (also treats a 400 invalid-worker-id as missing), and `ghostwriter.ts`
 * (also treats 5xx as unreachable).
 */
import { ApiError, GatewayTimeoutError } from "./types";

export function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof GatewayTimeoutError) return false;
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}
