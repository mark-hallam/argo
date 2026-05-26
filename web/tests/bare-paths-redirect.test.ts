import { describe, expect, it, vi } from "vitest";

const { redirectMock } = vi.hoisted(() => ({
  redirectMock: vi.fn((target: string) => {
    throw new Error(`NEXT_REDIRECT:${target}`);
  }),
}));

vi.mock("next/navigation", () => ({
  redirect: redirectMock,
}));

import ForPage from "@/app/for/page";
import SupportPage from "@/app/(app)/support/page";

describe("bare path redirects", () => {
  it("redirects /support to the support inbox", () => {
    expect(() => SupportPage()).toThrow("NEXT_REDIRECT:/support/inbox");
    expect(redirectMock).toHaveBeenCalledWith("/support/inbox");
  });

  it("redirects /for to the sales vertical landing page", () => {
    expect(() => ForPage()).toThrow("NEXT_REDIRECT:/for/sales");
    expect(redirectMock).toHaveBeenCalledWith("/for/sales");
  });
});
