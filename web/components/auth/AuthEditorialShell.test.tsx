import { describe, expect, it } from "vitest";
import { render } from "@testing-library/react";
import { AuthEditorialShell } from "./AuthEditorialShell";

describe("AuthEditorialShell", () => {
  it("declares a prefers-reduced-motion guard so the auth chrome freezes", () => {
    // Auth pages do not mount MotionRoot, so framer-motion's
    // reduced-motion swap never reaches this subtree. The CSS guard
    // is the only protection against radial-blob, SSO hover, and
    // field-focus transitions for users with the OS preference set.
    const { container } = render(
      <AuthEditorialShell
        topRightLabel="New here?"
        topRightHref="/auth/signup"
        topRightCta="Create account"
      >
        <div>auth body</div>
      </AuthEditorialShell>,
    );
    const styleNodes = container.ownerDocument.querySelectorAll("style");
    const css = Array.from(styleNodes)
      .map((s) => s.textContent ?? "")
      .join("\n");
    expect(css).toMatch(/prefers-reduced-motion/);
    expect(css).toMatch(/transition-duration:\s*0\.001ms/);
    expect(css).toMatch(/animation-iteration-count:\s*1/);
  });
});
