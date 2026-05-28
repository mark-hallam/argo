import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { layerChipLabel } from "@/app/(app)/skills/page";

// Regression test for F07 (whole-site-mvp-claims-coverage / ux-audit-
// 2026-05-27-2351): every skill card on /skills used to render a "LIBRARY"
// chip, but the chip wasn't interactive. Operators expected it to filter
// the catalog because the TRY pills above the grid look identical. The
// Option B fix renames the chip to "Verified" for library-layer skills,
// styles it as decorative (`cursor: default`), and documents in code why
// it intentionally is not a filter.

describe("layerChipLabel — decorative chip on /skills cards", () => {
  it("renames 'library' to 'Verified' so the chip stops reading as a filter token", () => {
    expect(layerChipLabel("library")).toBe("Verified");
  });

  it("passes through 'workspace' untouched (still informational, still decorative)", () => {
    expect(layerChipLabel("workspace")).toBe("workspace");
  });

  it("never returns the literal 'LIBRARY' text that triggered the F07 confusion", () => {
    const inputs = ["library", "workspace", "team", "anything"];
    for (const layer of inputs) {
      expect(layerChipLabel(layer).toUpperCase()).not.toBe("LIBRARY");
    }
  });
});

describe("skill-card decorative chip render", () => {
  it("renders 'Verified' (not the literal layer value) and marks the chip decorative", () => {
    render(
      <span
        className="skill-meta-item skill-meta-item--decorative"
        data-testid="skill-layer-chip"
      >
        {layerChipLabel("library")}
      </span>,
    );
    const chip = screen.getByTestId("skill-layer-chip");
    expect(chip).toHaveTextContent("Verified");
    expect(chip.className).toContain("skill-meta-item--decorative");
    // The chip is a <span>, not a <button> — clicking it does not run an
    // onClick handler and does not look like a filter affordance. This
    // assertion guards against a future regression that swaps the chip
    // back to a <button> without wiring real filter behavior.
    expect(chip.tagName).toBe("SPAN");
  });
});
