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

  it("pins the desktop editorial stage to the centered 720px container", () => {
    const { container } = render(
      <AuthEditorialShell
        topRightLabel="New here?"
        topRightHref="/auth/signup"
        topRightCta="Create account"
      >
        <form>auth form</form>
      </AuthEditorialShell>,
    );
    const css = Array.from(container.ownerDocument.querySelectorAll("style"))
      .map((s) => s.textContent ?? "")
      .join("\n");
    expect(css).toMatch(/\.auth-stage\s*{[^}]*box-sizing:\s*border-box/s);
    expect(css).toMatch(/\.auth-stage\s*{[^}]*max-width:\s*720px/s);
    expect(css).toMatch(/\.auth-stage\s*{[^}]*margin:\s*0 auto/s);
    expect(css).toMatch(/\.auth-stage\s*{[^}]*padding:\s*64px 24px 100px/s);
    expect(css).toMatch(/\.auth-stage\s*{[^}]*text-align:\s*center/s);
    expect(css).toMatch(/\.auth-stage form,[^}]*max-width:\s*460px/s);
    expect(css).toMatch(/\.field-block\s*{[^}]*text-align:\s*left/s);
  });
});
