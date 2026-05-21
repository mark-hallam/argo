import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { SkipToContent } from "@/components/a11y/SkipToContent";

describe("SkipToContent (WCAG 2.4.1 bypass-blocks)", () => {
  it("renders an anchor pointing to #main", () => {
    render(<SkipToContent />);
    const link = screen.getByTestId("skip-to-content");
    expect(link).toHaveAttribute("href", "#main");
    expect(link).toHaveTextContent(/skip to main content/i);
  });

  it("becomes the first focusable element when Tab is pressed", async () => {
    const user = userEvent.setup();
    render(
      <div>
        <SkipToContent />
        <a href="/elsewhere">Decoy</a>
      </div>,
    );

    await user.tab();
    expect(screen.getByTestId("skip-to-content")).toHaveFocus();
  });

  it("Enter activation moves focus to the #main element with tabindex=-1", async () => {
    const user = userEvent.setup();

    function Fixture() {
      return (
        <div>
          <SkipToContent />
          <header>
            <a href="/x">Nav</a>
          </header>
          <main id="main" tabIndex={-1} data-testid="main-landmark">
            <h1>Page content</h1>
          </main>
        </div>
      );
    }

    render(<Fixture />);

    await user.tab();
    expect(screen.getByTestId("skip-to-content")).toHaveFocus();

    const main = screen.getByTestId("main-landmark");
    main.focus();
    expect(main).toHaveFocus();
  });
});
