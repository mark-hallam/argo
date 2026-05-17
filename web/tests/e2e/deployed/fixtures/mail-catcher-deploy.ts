/**
 * Real-mail catcher for the deployed-real E2E lane.
 *
 * Unlike the local lane's in-memory MailCatcher — which intercepts the
 * SPA's `/api/auth/signup` POST and mints its own token — the deployed
 * lane has the real gateway send a real magic-link email. We need an
 * actual inbox to read it back from. Two ways operators commonly wire
 * this:
 *
 *   1. **mailcatcher.io / smtp4dev style**: JSON inbox over HTTP.
 *      Endpoint exposed via `ARGO_E2E_MAIL_CATCHER_URL`. Responses look
 *      like `[{ "to": "e2e-...@…", "html": "…<a href='https://…/verify?token=…'>…" }]`.
 *   2. **Postmark inbound** or similar: same shape, different host.
 *
 * Either way this fixture polls `${baseUrl}/inbox?to=<address>` until a
 * message arrives, then extracts the verify URL with a single regex.
 * No vendor-specific knobs — the operator's job is to shape the inbox
 * proxy to this contract (the runbook documents both setups).
 *
 * If nothing arrives within the timeout we throw with a clear
 * "operator-resume" hint so the run can be salvaged by pasting the
 * link from a real mail client (the throwaway address is unique per
 * run; nothing leaks between runs).
 */

import type { MagicLink } from "../../fixtures/mail-catcher";

interface InboxMessage {
  to?: string;
  recipients?: string[];
  html?: string;
  text?: string;
  body?: string;
}

const VERIFY_URL_RE = /https?:\/\/[^\s"'<>]+\/auth\/verify\?token=[A-Za-z0-9._\-+%]+/;

export class DeployMailCatcher {
  constructor(
    private readonly baseUrl: string,
    private readonly fetchImpl: typeof fetch = fetch,
  ) {}

  /**
   * Poll the inbox until `email` receives a message containing a verify
   * URL, or the deadline expires. The poll interval is 1s — fast enough
   * to keep specs snappy, slow enough not to hammer the operator's
   * inbox proxy.
   */
  async waitForLink(email: string, timeoutMs: number): Promise<MagicLink> {
    const deadline = Date.now() + timeoutMs;
    const target = email.toLowerCase();
    let lastError: unknown = null;
    while (Date.now() < deadline) {
      try {
        const res = await this.fetchImpl(
          `${this.baseUrl.replace(/\/+$/, "")}/inbox?to=${encodeURIComponent(email)}`,
          { headers: { accept: "application/json" } },
        );
        if (res.ok) {
          const messages = (await res.json()) as InboxMessage[];
          for (const m of messages) {
            if (!recipientMatches(m, target)) continue;
            const body = m.html ?? m.text ?? m.body ?? "";
            const match = VERIFY_URL_RE.exec(body);
            if (match) {
              const url = match[0];
              const tokenMatch = /token=([^&"'<>\s]+)/.exec(url);
              return {
                email,
                token: tokenMatch ? decodeURIComponent(tokenMatch[1]) : "",
                url,
              };
            }
          }
        }
      } catch (err) {
        lastError = err;
      }
      await new Promise((r) => setTimeout(r, 1000));
    }
    const detail = lastError instanceof Error ? `: ${lastError.message}` : "";
    throw new Error(
      `deploy mail-catcher: no magic link for ${email} within ${timeoutMs}ms (operator-resume: paste link manually)${detail}`,
    );
  }
}

function recipientMatches(message: InboxMessage, target: string): boolean {
  if (message.to && message.to.toLowerCase() === target) return true;
  if (Array.isArray(message.recipients)) {
    return message.recipients.some((r) => r.toLowerCase() === target);
  }
  return false;
}
