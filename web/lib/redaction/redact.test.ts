import { describe, expect, it } from "vitest";
import {
  POLICY_MASK_ALL,
  POLICY_SHOW_ALL,
  maskedCategories,
  policyMasksAnything,
  redactPII,
  resultHasMasks,
} from "./redact";
import { digitCount, luhnValid } from "./patterns";

describe("redactPII — email", () => {
  it("masks a single email with the [EMAIL] sentinel", () => {
    const { redacted, masked } = redactPII(
      "contact me at sam@acmeco.com",
      { ...POLICY_MASK_ALL },
    );
    expect(redacted).toBe("contact me at [EMAIL]");
    expect(masked.email).toBe(1);
  });

  it("returns the original text when the email policy is 'show'", () => {
    const { redacted, masked } = redactPII("ping kira+work@brand.co.nz", {
      ...POLICY_MASK_ALL,
      email: "show",
    });
    expect(redacted).toBe("ping kira+work@brand.co.nz");
    expect(masked.email).toBe(0);
  });

  it("masks multiple emails in one string", () => {
    const { redacted, masked } = redactPII(
      "a@x.co talked to b.c@y.io about prices",
      POLICY_MASK_ALL,
    );
    expect(redacted).toBe("[EMAIL] talked to [EMAIL] about prices");
    expect(masked.email).toBe(2);
  });
});

describe("redactPII — phone", () => {
  it("masks an NZ mobile-style number", () => {
    const { redacted, masked } = redactPII("call me on +64 21 555 0123", POLICY_MASK_ALL);
    expect(redacted).toBe("call me on [PHONE]");
    expect(masked.phone).toBe(1);
  });

  it("masks a US toll-free-style number with dashes", () => {
    const { redacted } = redactPII("dial 1-800-555-0199 for support", POLICY_MASK_ALL);
    expect(redacted).toBe("dial [PHONE] for support");
  });

  it("does NOT mask short digit runs (< 7 digits)", () => {
    const { redacted, masked } = redactPII("PIN 12345 in slot 3", POLICY_MASK_ALL);
    expect(redacted).toBe("PIN 12345 in slot 3");
    expect(masked.phone).toBe(0);
  });

  it("does NOT mask ISO dates (digit count too low)", () => {
    const { redacted, masked } = redactPII("delivered on 2026-05-11", POLICY_MASK_ALL);
    expect(masked.phone).toBe(0);
    expect(redacted).toContain("2026-05-11");
  });
});

describe("redactPII — card", () => {
  it("masks a Luhn-valid Visa test number", () => {
    // 4111 1111 1111 1111 is the canonical Visa test PAN; Luhn-valid.
    const { redacted, masked } = redactPII("paid with 4111 1111 1111 1111", POLICY_MASK_ALL);
    expect(redacted).toBe("paid with [CARD]");
    expect(masked.card).toBe(1);
  });

  it("rejects a 16-digit sequence that fails Luhn", () => {
    // Same shape, off-by-one to break the checksum.
    const { redacted, masked } = redactPII("order id 4111 1111 1111 1112", POLICY_MASK_ALL);
    // No mask from card; might still slip into phone? digitCount=16 > 15 so phone refuses.
    expect(masked.card).toBe(0);
    expect(redacted).toContain("4111 1111 1111 1112");
  });
});

describe("redactPII — IBAN + IRD", () => {
  it("masks an IBAN", () => {
    const { redacted, masked } = redactPII(
      "wire to DE89370400440532013000 by Friday",
      POLICY_MASK_ALL,
    );
    expect(redacted).toBe("wire to [IBAN] by Friday");
    expect(masked.iban).toBe(1);
  });

  it("masks an NZ IRD number (9-digit hyphenated)", () => {
    const { redacted, masked } = redactPII("IRD 123-456-789 on file", POLICY_MASK_ALL);
    expect(redacted).toBe("IRD [IRD] on file");
    expect(masked.ird).toBe(1);
  });

  it("masks an NZ IRD number (8-digit hyphenated)", () => {
    const { redacted } = redactPII("legacy IRD 12-345-678 still active", POLICY_MASK_ALL);
    expect(redacted).toContain("[IRD]");
  });
});

describe("redactPII — policy + helpers", () => {
  it("show-all policy returns the original text untouched", () => {
    const text = "email a@b.co, phone +64 21 555 0123, card 4111 1111 1111 1111";
    const { redacted, masked } = redactPII(text, POLICY_SHOW_ALL);
    expect(redacted).toBe(text);
    expect(resultHasMasks({ redacted, masked })).toBe(false);
  });

  it("masks only the categories whose policy says 'mask'", () => {
    const { redacted } = redactPII("a@b.co and +64 21 555 0123", {
      ...POLICY_SHOW_ALL,
      email: "mask",
    });
    expect(redacted).toBe("[EMAIL] and +64 21 555 0123");
  });

  it("policyMasksAnything / resultHasMasks / maskedCategories", () => {
    expect(policyMasksAnything(POLICY_MASK_ALL)).toBe(true);
    expect(policyMasksAnything(POLICY_SHOW_ALL)).toBe(false);

    const r = redactPII("a@b.co and IRD 123-456-789", POLICY_MASK_ALL);
    expect(resultHasMasks(r)).toBe(true);
    expect(maskedCategories(r).sort()).toEqual(["email", "ird"]);
  });
});

describe("patterns — Luhn + digitCount", () => {
  it("luhnValid accepts known-good Visa and Mastercard test pans", () => {
    expect(luhnValid("4111 1111 1111 1111")).toBe(true);
    expect(luhnValid("5500-0000-0000-0004")).toBe(true);
  });

  it("luhnValid rejects too-short / too-long sequences and non-digits", () => {
    expect(luhnValid("1234567890")).toBe(false); // too short
    expect(luhnValid("11111111111111111111")).toBe(false); // 20 digits
    expect(luhnValid("4111-abcd-1111-1111")).toBe(false);
  });

  it("digitCount ignores non-digit chars", () => {
    expect(digitCount("+64 21 555 0123")).toBe(11);
    expect(digitCount("no digits here")).toBe(0);
  });
});
