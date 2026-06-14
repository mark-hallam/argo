import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

// Mock the API client so the test exercises the form's render states + that it
// reuses the EXISTING enterprise-inquiry endpoint — not the network itself.
vi.mock("@/lib/api/enterprise-inquiry", async () => {
  const actual = await vi.importActual<
    typeof import("@/lib/api/enterprise-inquiry")
  >("@/lib/api/enterprise-inquiry");
  return {
    ...actual,
    // Keep the real isValidEmail; stub only the network call.
    submitEnterpriseInquiry: vi.fn(),
  };
});

import { submitEnterpriseInquiry } from "@/lib/api/enterprise-inquiry";
import { ApiError } from "@/lib/api/types";
import { WalkthroughCta } from "@/components/marketing/WalkthroughCta";

const submitMock = vi.mocked(submitEnterpriseInquiry);

afterEach(() => {
  vi.clearAllMocks();
  vi.unstubAllEnvs();
});

async function fillValidForm(user: ReturnType<typeof userEvent.setup>) {
  await user.type(screen.getByTestId("walkthrough-email"), "cto@acme.io");
  await user.type(screen.getByTestId("walkthrough-company"), "Acme Corp");
  await user.type(screen.getByTestId("walkthrough-name"), "Dana Quinn");
  await user.type(screen.getByTestId("walkthrough-role"), "CTO");
  await user.type(
    screen.getByTestId("walkthrough-handoff"),
    "Triage inbound security questionnaires",
  );
}

describe("WalkthroughCta", () => {
  it("renders the 'Book a governance walkthrough' CTA (no dead handler, no pre-faked success)", () => {
    render(<WalkthroughCta />);
    expect(screen.getByTestId("walkthrough-submit")).toHaveTextContent(
      /book a governance walkthrough/i,
    );
    // No success/scheduler/honest state before any submit.
    expect(screen.queryByTestId("walkthrough-success")).toBeNull();
    expect(screen.queryByTestId("walkthrough-scheduler")).toBeNull();
    expect(screen.queryByTestId("walkthrough-queued-honest")).toBeNull();
  });

  it("submits the qualifying form to the EXISTING /api/v1/enterprise-inquiry route", async () => {
    submitMock.mockResolvedValueOnce({ queued: true });
    const user = userEvent.setup();
    render(<WalkthroughCta source="landing-hero" />);

    await fillValidForm(user);
    await user.click(screen.getByTestId("walkthrough-submit"));

    await screen.findByTestId("walkthrough-success");
    expect(submitMock).toHaveBeenCalledTimes(1);
    const arg = submitMock.mock.calls[0][0];
    expect(arg.company).toBe("Acme Corp");
    expect(arg.name).toBe("Dana Quinn");
    expect(arg.email).toBe("cto@acme.io");
    // Role + the free-text handoff are folded into the inquiry message so the
    // reused endpoint carries the call's context.
    expect(arg.message).toContain("Role: CTO");
    expect(arg.message).toContain(
      "Triage inbound security questionnaires",
    );
  });

  it("renders the scheduler embed on success WHEN NEXT_PUBLIC_WALKTHROUGH_SCHEDULER_URL is set", async () => {
    vi.stubEnv(
      "NEXT_PUBLIC_WALKTHROUGH_SCHEDULER_URL",
      "https://cal.com/skycot/governance-walkthrough",
    );
    submitMock.mockResolvedValueOnce({ queued: true });
    const user = userEvent.setup();
    render(<WalkthroughCta />);

    await fillValidForm(user);
    await user.click(screen.getByTestId("walkthrough-submit"));

    const scheduler = await screen.findByTestId("walkthrough-scheduler");
    expect(scheduler).toBeInTheDocument();
    const embed = screen.getByTestId("walkthrough-scheduler-embed");
    expect(embed.tagName).toBe("IFRAME");
    expect(embed).toHaveAttribute(
      "src",
      "https://cal.com/skycot/governance-walkthrough",
    );
    // The honest "we'll be in touch" state must NOT show when a real calendar
    // is wired.
    expect(screen.queryByTestId("walkthrough-queued-honest")).toBeNull();
  });

  it("NO-THEATER: renders the honest 'we'll be in touch' state and NO booking affordance when the scheduler URL is unset", async () => {
    // Env var unset → graceful degrade. The form submission alone IS the inquiry.
    vi.stubEnv("NEXT_PUBLIC_WALKTHROUGH_SCHEDULER_URL", "");
    submitMock.mockResolvedValueOnce({ queued: true });
    const user = userEvent.setup();
    render(<WalkthroughCta />);

    await fillValidForm(user);
    await user.click(screen.getByTestId("walkthrough-submit"));

    const honest = await screen.findByTestId("walkthrough-queued-honest");
    expect(honest).toHaveTextContent(/we’ll be in touch to schedule/i);
    // Never promise a calendar that isn't wired (the dead-play-button lesson).
    expect(screen.queryByTestId("walkthrough-scheduler")).toBeNull();
    expect(screen.queryByTestId("walkthrough-scheduler-embed")).toBeNull();
    expect(document.querySelector("iframe")).toBeNull();
  });

  it("blocks an invalid work email client-side without calling the route", async () => {
    const user = userEvent.setup();
    render(<WalkthroughCta />);

    await user.type(screen.getByTestId("walkthrough-email"), "nope");
    await user.type(screen.getByTestId("walkthrough-company"), "Acme Corp");
    await user.type(screen.getByTestId("walkthrough-name"), "Dana Quinn");
    await user.click(screen.getByTestId("walkthrough-submit"));

    await waitFor(() =>
      expect(screen.getByTestId("walkthrough-error")).toHaveTextContent(
        /valid work email/i,
      ),
    );
    expect(submitMock).not.toHaveBeenCalled();
  });
});
