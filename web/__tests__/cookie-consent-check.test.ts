import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  canTrackAnalytics,
  canTrackMarketing,
  readSpaConsent,
} from "@/lib/cookie-consent-check";

const COOKIE_NAME = "cookieconsent_v1";

function installMemoryStorage() {
  const data = new Map<string, string>();
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: {
      get length() {
        return data.size;
      },
      clear: () => data.clear(),
      getItem: (k: string) => (data.has(k) ? (data.get(k) as string) : null),
      key: (i: number) => Array.from(data.keys())[i] ?? null,
      removeItem: (k: string) => {
        data.delete(k);
      },
      setItem: (k: string, v: string) => {
        data.set(k, String(v));
      },
    },
  });
}

function clearCookies() {
  for (const part of document.cookie.split(";")) {
    const name = part.split("=")[0]?.trim();
    if (name) {
      document.cookie = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/`;
    }
  }
}

beforeEach(() => {
  installMemoryStorage();
  clearCookies();
});

afterEach(() => {
  window.localStorage.clear();
  clearCookies();
});

describe("cookie-consent-check", () => {
  it("returns null when no consent is recorded", () => {
    expect(readSpaConsent()).toBeNull();
    expect(canTrackAnalytics()).toBe(false);
    expect(canTrackMarketing()).toBe(false);
  });

  it("reads consent from localStorage when present", () => {
    const choice = {
      essential: true,
      analytics: true,
      marketing: false,
      decided: true,
      timestamp: Date.now(),
    };
    window.localStorage.setItem(COOKIE_NAME, JSON.stringify(choice));
    const read = readSpaConsent();
    expect(read?.analytics).toBe(true);
    expect(read?.marketing).toBe(false);
    expect(read?.decided).toBe(true);
    expect(canTrackAnalytics()).toBe(true);
    expect(canTrackMarketing()).toBe(false);
  });

  it("falls back to cookie when localStorage is empty", () => {
    const choice = {
      essential: true,
      analytics: false,
      marketing: true,
      decided: true,
      timestamp: Date.now(),
    };
    document.cookie = `${COOKIE_NAME}=${encodeURIComponent(JSON.stringify(choice))}; path=/`;
    const read = readSpaConsent();
    expect(read?.analytics).toBe(false);
    expect(read?.marketing).toBe(true);
    expect(canTrackAnalytics()).toBe(false);
    expect(canTrackMarketing()).toBe(true);
  });

  it("treats undecided consent as no-track even with categories true", () => {
    const choice = {
      essential: true,
      analytics: true,
      marketing: true,
      decided: false,
      timestamp: 0,
    };
    window.localStorage.setItem(COOKIE_NAME, JSON.stringify(choice));
    expect(canTrackAnalytics()).toBe(false);
    expect(canTrackMarketing()).toBe(false);
  });

  it("ignores malformed JSON gracefully", () => {
    window.localStorage.setItem(COOKIE_NAME, "{not json");
    expect(readSpaConsent()).toBeNull();
    expect(canTrackAnalytics()).toBe(false);
  });
});
