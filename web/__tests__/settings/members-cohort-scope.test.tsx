import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import MembersPage from "@/app/(app)/settings/members/page";
import { CohortScopeDeniedEmptyState } from "@/components/skycot/CohortScopeDeniedEmptyState";

// `fetchCohorts` falls back to `SAMPLE_COHORT_SUMMARIES` when the
// network request fails (404 or TypeError). The vi.fn() global fetch
// below returns undefined → TypeError → mock fallback fires.

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

beforeEach(() => {
  global.fetch = vi.fn();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("/settings/members cohort scope", () => {
  it("renders an 'All cohorts' chip for owner rows", async () => {
    render(wrap(<MembersPage />));
    const ownerRow = await screen.findByTestId("member-row-mbr_sam");
    expect(ownerRow.textContent).toContain("All cohorts");
  });

  it("renders specific cohort chips for scoped users", async () => {
    render(wrap(<MembersPage />));
    const diegoRow = await screen.findByTestId("member-row-mbr_diego");
    await waitFor(() => {
      expect(diegoRow.textContent).toContain("Sales");
    });
  });

  it("opens the Edit Member modal with cohort picker", async () => {
    const user = userEvent.setup();
    render(wrap(<MembersPage />));
    await user.click(await screen.findByTestId("member-edit-mbr_diego"));
    expect(await screen.findByTestId("edit-member-modal")).toBeInTheDocument();
    expect(screen.getByTestId("cohort-scope-picker")).toBeInTheDocument();
    expect(screen.getByTestId("cohort-scope-toggle-all")).toBeInTheDocument();
    expect(screen.getByTestId("cohort-scope-toggle-specific")).toBeInTheDocument();
  });

  it("disables the picker for admin (override) and re-enables on role downgrade", async () => {
    const user = userEvent.setup();
    render(wrap(<MembersPage />));
    await user.click(await screen.findByTestId("member-edit-mbr_olivia"));
    expect(await screen.findByTestId("cohort-scope-override-note")).toBeInTheDocument();
    expect(screen.getByTestId("cohort-scope-toggle-all")).toBeDisabled();

    await user.click(screen.getByTestId("edit-role-user"));
    await waitFor(() => {
      expect(screen.queryByTestId("cohort-scope-override-note")).not.toBeInTheDocument();
    });
    expect(screen.getByTestId("cohort-scope-toggle-all")).not.toBeDisabled();
  });

  it("persists cohort-scope changes back to the member row", async () => {
    const user = userEvent.setup();
    render(wrap(<MembersPage />));
    await user.click(await screen.findByTestId("member-edit-mbr_diego"));

    // Switch to specific cohorts mode (already specific) then add support::trusted.
    await user.click(screen.getByTestId("cohort-scope-toggle-specific"));
    await user.click(await screen.findByTestId("cohort-scope-chip-support::trusted"));
    await user.click(screen.getByTestId("edit-save-btn"));

    const diegoRow = await screen.findByTestId("member-row-mbr_diego");
    await waitFor(() => {
      expect(diegoRow.textContent).toContain("Support");
    });
  });

  it("invite modal exposes the cohort-scope picker", async () => {
    const user = userEvent.setup();
    render(wrap(<MembersPage />));
    await user.click(screen.getByText("Invite teammate"));
    expect(await screen.findByTestId("invite-member-modal")).toBeInTheDocument();
    expect(screen.getByTestId("cohort-scope-picker")).toBeInTheDocument();
  });
});

describe("CohortScopeDeniedEmptyState", () => {
  it("renders a soft-403 message for workers", () => {
    render(<CohortScopeDeniedEmptyState resource="worker" cohortLabel="Sales · Established" />);
    expect(screen.getByTestId("cohort-scope-denied")).toBeInTheDocument();
    expect(
      screen.getByText(/isn't in a cohort you have access to/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/Sales · Established/)).toBeInTheDocument();
  });

  it("exposes an optional Request access CTA", async () => {
    const user = userEvent.setup();
    const onRequest = vi.fn();
    render(
      <CohortScopeDeniedEmptyState resource="audit entry" onRequestAccess={onRequest} />,
    );
    await user.click(screen.getByText("Request access"));
    expect(onRequest).toHaveBeenCalledOnce();
  });
});
