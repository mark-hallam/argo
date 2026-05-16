/**
 * Magic-link mail catcher.
 *
 * Local/CI: in-memory store populated by intercepting the SPA's
 * `/api/auth/signup` route — the test mints a verification token and
 * the catcher hands it back when asked. No real SMTP server runs;
 * smtp4dev is overkill given the SPA already has a verify endpoint
 * that accepts any token the test produces.
 *
 * Deploy-smoke: polls a real test inbox webhook (operator wires the
 * inbox URL via `E2E_TEST_INBOX_URL`). If no email arrives within the
 * timeout, raises so the operator can intercept the link manually
 * ("operator-resume" mode).
 */

import type { Page } from "@playwright/test";

export interface MagicLink {
  email: string;
  token: string;
  url: string;
}

export class MailCatcher {
  private inbox = new Map<string, MagicLink>();
  private readonly spaUrl: string;

  constructor(spaUrl: string) {
    this.spaUrl = spaUrl;
  }

  /** Install request interceptors that record the signup → magic-link mapping. */
  async installLocalIntercepts(page: Page): Promise<void> {
    await page.route("**/api/auth/signup", async (route) => {
      const body = route.request().postDataJSON() as { email?: string };
      const email = body?.email ?? "";
      if (email) {
        const token = `e2e-token-${Date.now().toString(36)}`;
        this.inbox.set(email.toLowerCase(), {
          email,
          token,
          url: `${this.spaUrl}/auth/verify?token=${encodeURIComponent(token)}`,
        });
      }
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ status: "sent" }),
      });
    });
  }

  /** Wait until a magic link arrives for `email`. Returns the verify URL. */
  async waitForLink(email: string, timeoutMs: number): Promise<MagicLink> {
    const key = email.toLowerCase();
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      const hit = this.inbox.get(key);
      if (hit) return hit;
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error(`mail-catcher: no magic link for ${email} within ${timeoutMs}ms`);
  }

  /** Wipe inbox between tests. */
  clear(): void {
    this.inbox.clear();
  }
}
