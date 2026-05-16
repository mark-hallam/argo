import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TestimonialCard } from "../TestimonialCard";

describe("TestimonialCard", () => {
  it("renders quote, name, role, company, and an accessible avatar", () => {
    render(
      <TestimonialCard
        quote="Skycot replaced four scattered ops tools."
        name="Alex Rivera"
        role="Head of Operations"
        company="Northwind"
      />,
    );
    expect(screen.getByText(/Skycot replaced four scattered ops tools/)).toBeInTheDocument();
    expect(screen.getByText("Alex Rivera")).toBeInTheDocument();
    expect(screen.getByText(/Head of Operations/)).toBeInTheDocument();
    expect(screen.getByText(/Northwind/)).toBeInTheDocument();
    expect(screen.getByTestId("testimonial-card")).toBeInTheDocument();
  });
});
