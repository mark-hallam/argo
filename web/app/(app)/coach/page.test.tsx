import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import CoachPage from "./page";

describe("CoachPage (Cora founder coach)", () => {
  it("renders Cora's surface with both conversation modes", () => {
    render(<CoachPage />);
    expect(screen.getByRole("heading", { name: /Cora/i })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Coaching/i })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Founder interview/i })).toBeTruthy();
  });

  it("defaults to coaching mode (pressed)", () => {
    render(<CoachPage />);
    const coach = screen.getByRole("button", { name: /Coaching/i });
    expect(coach.getAttribute("aria-pressed")).toBe("true");
  });

  it("switches the active blurb when Founder interview is chosen", () => {
    render(<CoachPage />);
    fireEvent.click(screen.getByRole("button", { name: /Founder interview/i }));
    expect(screen.getByText(/draw out your founder story/i)).toBeTruthy();
  });
});
