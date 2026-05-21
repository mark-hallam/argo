import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import MembersPage from "@/app/(app)/settings/members/page";
import {
  EXPIRY_PRESETS,
  EXPIRY_PRESET_LABELS,
  TIME_BOUND_AUDIT_EVENTS,
  defaultCustomDate,
  expiryStatus,
  presetToExpiresAt,
  shouldShowExpiryBanner,
  timeBoundTooltip,
} from "@/lib/members/expiry";

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

beforeEach(() => {
  global.fetch = vi.fn();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("expiryStatus()", () => {
  const now = new Date("2026-05-21T12:00:00Z");

  it("returns green tone for >7 days remaining", () => {
    const expires = new Date("2026-06-10T12:00:00Z").toISOString();
    const s = expiryStatus(expires, now);
    expect(s.tone).toBe("green");
    expect(s.expired).toBe(false);
    expect(s.days).toBeGreaterThan(7);
    expect(s.label).toContain("Expires in");
  });

  it("returns amber tone within 7 days", () => {
    const expires = new Date("2026-05-25T12:00:00Z").toISOString();
    const s = expiryStatus(expires, now);
    expect(s.tone).toBe("amber");
    expect(s.expired).toBe(false);
    expect(s.days).toBeLessThanOrEqual(7);
  });

  it("returns red tone + expired=true for past dates", () => {
    const expires = new Date("2026-05-19T12:00:00Z").toISOString();
    const s = expiryStatus(expires, now);
    expect(s.tone).toBe("red");
    expect(s.expired).toBe(true);
    expect(s.days).toBeLessThan(0);
    expect(s.label).toContain("Expired");
  });

  it("returns 'Expires today' when 0 days remain (same instant)", () => {
    const expires = new Date("2026-05-21T12:00:00Z").toISOString();
    const s = expiryStatus(expires, now);
    expect(s.label).toBe("Expires today");
  });

  it("handles invalid ISO gracefully", () => {
    const s = expiryStatus("not-a-date", now);
    expect(s.tone).toBe("red");
    expect(s.expired).toBe(true);
  });
});

describe("shouldShowExpiryBanner()", () => {
  const now = new Date("2026-05-21T12:00:00Z");

  it("returns false for null/undefined", () => {
    expect(shouldShowExpiryBanner(null, now)).toBe(false);
    expect(shouldShowExpiryBanner(undefined, now)).toBe(false);
  });

  it("returns false for >7 days remaining", () => {
    const expires = new Date("2026-06-15T12:00:00Z").toISOString();
    expect(shouldShowExpiryBanner(expires, now)).toBe(false);
  });

  it("returns true for ≤7 days remaining", () => {
    const expires = new Date("2026-05-25T12:00:00Z").toISOString();
    expect(shouldShowExpiryBanner(expires, now)).toBe(true);
  });

  it("returns false for past dates", () => {
    const expires = new Date("2026-05-19T12:00:00Z").toISOString();
    expect(shouldShowExpiryBanner(expires, now)).toBe(false);
  });
});

describe("presetToExpiresAt()", () => {
  const now = new Date("2026-05-21T12:00:00Z");

  it("returns null for 'never' preset", () => {
    expect(presetToExpiresAt("never", null, now)).toBeNull();
  });

  it("returns ~7 days hence for '7d' preset", () => {
    const iso = presetToExpiresAt("7d", null, now);
    expect(iso).not.toBeNull();
    const d = Date.parse(iso!) - now.getTime();
    expect(Math.round(d / (24 * 60 * 60 * 1000))).toBe(7);
  });

  it("returns ~30 days hence for '30d' preset", () => {
    const iso = presetToExpiresAt("30d", null, now);
    const d = Date.parse(iso!) - now.getTime();
    expect(Math.round(d / (24 * 60 * 60 * 1000))).toBe(30);
  });

  it("returns ISO from customDate for 'custom' preset", () => {
    const iso = presetToExpiresAt("custom", "2026-12-25", now);
    expect(iso).not.toBeNull();
    expect(iso!.slice(0, 10)).toBe("2026-12-25");
  });

  it("returns null for 'custom' with missing date", () => {
    expect(presetToExpiresAt("custom", null, now)).toBeNull();
  });

  it("returns null for 'custom' with invalid date", () => {
    expect(presetToExpiresAt("custom", "garbage", now)).toBeNull();
  });
});

describe("defaultCustomDate()", () => {
  it("returns yyyy-mm-dd 30 days hence", () => {
    const now = new Date("2026-05-21T12:00:00Z");
    expect(defaultCustomDate(now)).toBe("2026-06-20");
  });
});

describe("timeBoundTooltip()", () => {
  const now = new Date("2026-05-21T12:00:00Z");

  it("returns null for null/undefined", () => {
    expect(timeBoundTooltip(null, now)).toBeNull();
    expect(timeBoundTooltip(undefined, now)).toBeNull();
  });

  it("uses 'expires' wording when still valid", () => {
    const expires = new Date("2026-05-25T12:00:00Z").toISOString();
    expect(timeBoundTooltip(expires, now)).toContain("expires");
  });

  it("uses 'expired' wording when past", () => {
    const expires = new Date("2026-05-19T12:00:00Z").toISOString();
    expect(timeBoundTooltip(expires, now)).toContain("expired");
  });
});

describe("audit event catalogue", () => {
  it("exposes the three expected event names", () => {
    expect(TIME_BOUND_AUDIT_EVENTS.inviteSent).toBe("member.invite-sent.time-bound");
    expect(TIME_BOUND_AUDIT_EVENTS.accessExtended).toBe(
      "member.access-extended.time-bound",
    );
    expect(TIME_BOUND_AUDIT_EVENTS.accessExpired).toBe(
      "member.access-expired.time-bound",
    );
  });
});

describe("invite modal — Access expires field", () => {
  it("renders all five preset radios", async () => {
    const user = userEvent.setup();
    render(wrap(<MembersPage />));
    await user.click(await screen.findByRole("button", { name: /invite teammate/i }));
    expect(await screen.findByTestId("invite-member-modal")).toBeInTheDocument();
    for (const p of EXPIRY_PRESETS) {
      const btn = screen.getByTestId(`invite-expiry-${p}`);
      expect(btn).toHaveTextContent(EXPIRY_PRESET_LABELS[p]);
    }
  });

  it("shows the custom date picker only when 'Custom' is selected", async () => {
    const user = userEvent.setup();
    render(wrap(<MembersPage />));
    await user.click(await screen.findByRole("button", { name: /invite teammate/i }));
    await screen.findByTestId("invite-member-modal");
    expect(screen.queryByTestId("invite-expiry-date")).not.toBeInTheDocument();
    await user.click(screen.getByTestId("invite-expiry-custom"));
    expect(await screen.findByTestId("invite-expiry-date")).toBeInTheDocument();
  });

  it("renders the helper text", async () => {
    const user = userEvent.setup();
    render(wrap(<MembersPage />));
    await user.click(await screen.findByRole("button", { name: /invite teammate/i }));
    const body = await screen.findByTestId("invite-member-modal");
    expect(body.textContent).toContain("auditors, external consultants");
    expect(body.textContent).toContain("lose access automatically");
  });
});

describe("member list — expiry pill", () => {
  it("renders a pill for time-bound seed members", async () => {
    render(wrap(<MembersPage />));
    const naomi = await screen.findByTestId("member-row-mbr_naomi");
    await waitFor(() => {
      const pill = naomi.querySelector('[data-testid="member-expiry-pill"]');
      expect(pill).not.toBeNull();
      expect(pill?.getAttribute("data-tone")).toBe("green");
    });
  });

  it("renders no pill for permanent members", async () => {
    render(wrap(<MembersPage />));
    const sam = await screen.findByTestId("member-row-mbr_sam");
    expect(sam.querySelector('[data-testid="member-expiry-pill"]')).toBeNull();
  });

  it("renders amber pill for ≤7-day expiry", async () => {
    render(wrap(<MembersPage />));
    const jordan = await screen.findByTestId("member-row-mbr_jordan");
    await waitFor(() => {
      const pill = jordan.querySelector('[data-testid="member-expiry-pill"]');
      expect(pill?.getAttribute("data-tone")).toBe("amber");
    });
  });

  it("renders red pill + grays out the row for expired members", async () => {
    render(wrap(<MembersPage />));
    const lee = await screen.findByTestId("member-row-mbr_lee");
    await waitFor(() => {
      const pill = lee.querySelector('[data-testid="member-expiry-pill"]');
      expect(pill?.getAttribute("data-tone")).toBe("red");
    });
    expect(lee.getAttribute("data-expired")).toBe("true");
  });
});

describe("edit member modal — expiry field", () => {
  it("opens with 'never' selected for permanent members", async () => {
    const user = userEvent.setup();
    render(wrap(<MembersPage />));
    await user.click(await screen.findByTestId("member-edit-mbr_sam"));
    await screen.findByTestId("edit-member-modal");
    expect(screen.getByTestId("edit-expiry-never")).toHaveAttribute(
      "aria-checked",
      "true",
    );
  });

  it("opens with 'custom' selected for time-bound members", async () => {
    const user = userEvent.setup();
    render(wrap(<MembersPage />));
    await user.click(await screen.findByTestId("member-edit-mbr_naomi"));
    await screen.findByTestId("edit-member-modal");
    expect(screen.getByTestId("edit-expiry-custom")).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByTestId("edit-expiry-date")).toBeInTheDocument();
  });
});
