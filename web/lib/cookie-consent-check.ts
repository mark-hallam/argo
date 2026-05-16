// SPA-side consent check. Mirrors the cookie + localStorage contract
// owned by the marketing site's CookieBanner. Used to gate analytics
// events posted from the SPA so we honor the choice made on skycot.com.

const COOKIE_NAME = "cookieconsent_v1";

export interface SpaConsentChoice {
  essential: boolean;
  analytics: boolean;
  marketing: boolean;
  decided: boolean;
  timestamp: number;
}

function safeParse(raw: string): SpaConsentChoice | null {
  try {
    const obj = JSON.parse(raw);
    if (typeof obj !== "object" || obj === null) return null;
    return {
      essential: true,
      analytics: Boolean(obj.analytics),
      marketing: Boolean(obj.marketing),
      decided: Boolean(obj.decided),
      timestamp: typeof obj.timestamp === "number" ? obj.timestamp : 0,
    };
  } catch {
    return null;
  }
}

export function readSpaConsent(): SpaConsentChoice | null {
  if (typeof window === "undefined") return null;
  try {
    const ls = window.localStorage.getItem(COOKIE_NAME);
    if (ls) {
      const parsed = safeParse(ls);
      if (parsed) return parsed;
    }
  } catch {
    /* fall through */
  }
  if (typeof document === "undefined") return null;
  const match = document.cookie.match(new RegExp(`(?:^|; )${COOKIE_NAME}=([^;]+)`));
  if (!match) return null;
  return safeParse(decodeURIComponent(match[1]));
}

export function canTrackAnalytics(): boolean {
  const c = readSpaConsent();
  return c?.decided === true && c.analytics === true;
}

export function canTrackMarketing(): boolean {
  const c = readSpaConsent();
  return c?.decided === true && c.marketing === true;
}
