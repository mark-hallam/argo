import { afterEach, describe, expect, it } from "vitest";
import { readCsrfTokenFromCookie } from "./csrf-token";

const ORIGINAL_COOKIE = Object.getOwnPropertyDescriptor(
  Document.prototype,
  "cookie",
);

function stubCookie(value: string) {
  Object.defineProperty(document, "cookie", {
    configurable: true,
    get: () => value,
    set: () => {},
  });
}

describe("readCsrfTokenFromCookie", () => {
  afterEach(() => {
    if (ORIGINAL_COOKIE) {
      Object.defineProperty(Document.prototype, "cookie", ORIGINAL_COOKIE);
    }
  });

  it("reads the strict __Host-csrf cookie when present", () => {
    stubCookie("__Host-csrf=strict-tok; other=ignored");
    expect(readCsrfTokenFromCookie()).toBe("strict-tok");
  });

  it("falls back to the plain dev `csrf` cookie when __Host-csrf is absent", () => {
    // http://localhost dev origin: only the non-`__Host-` cookie
    // round-trips, so the helper must still resolve the token.
    stubCookie("csrf=dev-tok; foo=bar");
    expect(readCsrfTokenFromCookie()).toBe("dev-tok");
  });

  it("prefers __Host-csrf over the dev cookie when both are set", () => {
    stubCookie("__Host-csrf=strict-tok; csrf=dev-tok");
    expect(readCsrfTokenFromCookie()).toBe("strict-tok");
  });

  it("returns undefined when no CSRF cookie is set", () => {
    stubCookie("session=abc; foo=bar");
    expect(readCsrfTokenFromCookie()).toBeUndefined();
  });
});
