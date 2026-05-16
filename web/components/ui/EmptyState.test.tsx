import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { EmptyState } from "./EmptyState";
import { WorkersIllustration } from "@/components/empty-state/illustrations/WorkersIllustration";

describe("EmptyState", () => {
  it("renders headline, body, illustration, and CTA in default variant", () => {
    render(
      <EmptyState
        illustration={WorkersIllustration}
        headline="No teammates yet"
        body="Hire your first AI worker to start shipping."
        cta={{ label: "Hire a worker", href: "/hire" }}
      />,
    );

    const section = screen.getByRole("region", { name: "No teammates yet" });
    expect(section).toBeInTheDocument();
    expect(section.tagName).toBe("SECTION");

    const heading = screen.getByRole("heading", { name: "No teammates yet", level: 2 });
    expect(heading).toBeInTheDocument();

    expect(screen.getByText("Hire your first AI worker to start shipping.")).toBeInTheDocument();
    const cta = screen.getByRole("link", { name: "Hire a worker" });
    expect(cta).toHaveAttribute("href", "/hire");
  });

  it("renders compact variant without illustration", () => {
    render(
      <EmptyState
        variant="compact"
        illustration={WorkersIllustration}
        headline="Memory is quiet"
        body="When Maya remembers facts or decisions, they appear here."
      />,
    );
    expect(screen.getByRole("heading", { name: "Memory is quiet" })).toBeInTheDocument();
    expect(document.querySelector("svg")).toBeNull();
  });

  it("renders inline variant as a single-line section", () => {
    render(
      <EmptyState
        variant="inline"
        headline="Nothing to show"
        body="Try widening the time range."
      />,
    );
    const section = screen.getByRole("region", { name: "Nothing to show" });
    expect(section).toBeInTheDocument();
    expect(document.querySelector("svg")).toBeNull();
  });

  it("invokes onClick on secondary action button", async () => {
    const onClick = vi.fn();
    render(
      <EmptyState
        headline="No cohorts"
        body="Group similar tasks into a cohort."
        cta={{ label: "Create cohort", onClick }}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: "Create cohort" }));
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("forwards aria-label override and data-testid", () => {
    render(
      <EmptyState
        headline="Quiet day"
        ariaLabel="Audit timeline empty"
        data-testid="audit-empty-pristine"
      />,
    );
    expect(screen.getByRole("region", { name: "Audit timeline empty" })).toBeInTheDocument();
    expect(screen.getByTestId("audit-empty-pristine")).toBeInTheDocument();
  });
});
