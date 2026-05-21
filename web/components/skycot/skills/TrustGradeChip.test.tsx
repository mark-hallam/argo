import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TrustGradeChip } from "./TrustGradeChip";

describe("TrustGradeChip", () => {
  it("renders grade A (success tone) for builtin and published_verified", () => {
    const { rerender } = render(<TrustGradeChip grade="builtin" />);
    const chip = screen.getByTestId("skill-trust-grade");
    expect(chip).toHaveTextContent("Trust · A");
    expect(chip).toHaveAttribute("data-tone", "success");
    expect(chip).toHaveAttribute("data-letter", "A");
    expect(chip).toHaveAttribute("data-grade", "builtin");

    rerender(<TrustGradeChip grade="published_verified" />);
    const verified = screen.getByTestId("skill-trust-grade");
    expect(verified).toHaveTextContent("Trust · A");
    expect(verified).toHaveAttribute("data-tone", "success");
    expect(verified).toHaveAttribute("data-grade", "published_verified");
  });

  it("renders grade B (warning tone) for tenant", () => {
    render(<TrustGradeChip grade="tenant" />);
    const chip = screen.getByTestId("skill-trust-grade");
    expect(chip).toHaveTextContent("Trust · B");
    expect(chip).toHaveAttribute("data-tone", "warning");
    expect(chip).toHaveAttribute("data-letter", "B");
    expect(chip).toHaveAttribute("data-grade", "tenant");
  });

  it("renders 'Ungraded' in muted gray when grade is null", () => {
    render(<TrustGradeChip grade={null} />);
    const chip = screen.getByTestId("skill-trust-grade");
    expect(chip).toHaveTextContent("Ungraded");
    expect(chip).toHaveAttribute("data-tone", "muted");
    expect(chip).toHaveAttribute("data-grade", "ungraded");
    expect(chip).not.toHaveTextContent("Trust · ");
  });

  it("renders 'Ungraded' for an unknown grade discriminant", () => {
    render(<TrustGradeChip grade="totally-not-a-grade" />);
    const chip = screen.getByTestId("skill-trust-grade");
    expect(chip).toHaveTextContent("Ungraded");
    expect(chip).toHaveAttribute("data-tone", "muted");
  });

  it("respects the testId override", () => {
    render(<TrustGradeChip grade="learned_unreviewed" testId="custom-chip" />);
    const chip = screen.getByTestId("custom-chip");
    expect(chip).toHaveTextContent("Trust · D");
    expect(chip).toHaveAttribute("data-tone", "destructive");
  });
});
