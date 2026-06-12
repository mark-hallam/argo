import { describe, expect, it } from "vitest";
import { deriveInitials } from "./Avatar";

describe("deriveInitials", () => {
  it("takes the first letter of the first two words for multi-word names", () => {
    expect(deriveInitials("Ada Lovelace")).toBe("AL");
    expect(deriveInitials("Priya Singh Rao")).toBe("PS");
  });

  it("takes the first TWO letters for a single-word name, never a bare letter (F13)", () => {
    // A fresh hire named "Ivy" must render "IV", not a lone "I" — a
    // single letter reads as a placeholder, not a designed monogram.
    expect(deriveInitials("Ivy")).toBe("IV");
    expect(deriveInitials("maya")).toBe("MA");
  });

  it("degrades to '?' for empty or whitespace-only input", () => {
    expect(deriveInitials("")).toBe("?");
    expect(deriveInitials("   ")).toBe("?");
  });
});
