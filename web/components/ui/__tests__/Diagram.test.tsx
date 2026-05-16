import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Diagram } from "../Diagram";

describe("Diagram", () => {
  it("wraps content with an accessible region and renders the caption", () => {
    render(
      <Diagram ariaLabel="System architecture" caption="Fig 1. Skycot architecture">
        <svg viewBox="0 0 10 10" />
      </Diagram>,
    );
    expect(screen.getByRole("img", { name: "System architecture" })).toBeInTheDocument();
    expect(screen.getByText("Fig 1. Skycot architecture")).toBeInTheDocument();
  });
});
