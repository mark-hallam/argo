import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  MAX_DURATION_S,
  MAX_SPRING_SCALE,
  motion,
  reducedMotionFallbacks,
  toVariants,
} from "./variants";
import { MotionRoot, useMotion } from "./provider";

function pickDuration(t: unknown): number | undefined {
  if (!t || typeof t !== "object") return undefined;
  const d = (t as { duration?: unknown }).duration;
  return typeof d === "number" ? d : undefined;
}

describe("motion vocabulary", () => {
  const variantKeys = ["fadeUp", "fadeIn", "scaleIn", "successPulse", "springModal", "crossfade"] as const;

  it("exports the documented vocabulary surface", () => {
    for (const key of [...variantKeys, "staggerChildren"]) {
      expect(motion).toHaveProperty(key);
      expect(reducedMotionFallbacks).toHaveProperty(key);
    }
  });

  it("respects the 400ms ceiling on every variant", () => {
    for (const key of variantKeys) {
      const d = pickDuration(motion[key].transition);
      if (d !== undefined) expect(d).toBeLessThanOrEqual(MAX_DURATION_S);
    }
    const pulseTransition = (motion.successPulse.animate as { transition?: unknown }).transition;
    expect(pickDuration(pulseTransition) ?? 0).toBeLessThanOrEqual(MAX_DURATION_S);
  });

  it("keeps spring scale at or below 1.05", () => {
    const scale = (motion.successPulse.animate as { scale?: number[] }).scale;
    if (scale) for (const s of scale) expect(s).toBeLessThanOrEqual(MAX_SPRING_SCALE);
    const modalScale = (motion.springModal.animate as { scale?: number }).scale ?? 1;
    expect(modalScale).toBeLessThanOrEqual(MAX_SPRING_SCALE);
  });

  it("collapses every reduced-motion fallback to zero duration", () => {
    for (const key of variantKeys) {
      const d = pickDuration(reducedMotionFallbacks[key].transition);
      expect(d ?? 0).toBe(0);
    }
  });

  it("toVariants includes only the defined keys", () => {
    expect(toVariants(motion.fadeUp)).toEqual({
      initial: { opacity: 0, y: 8 },
      animate: { opacity: 1, y: 0 },
      exit: { opacity: 0, y: 8 },
    });
    expect(toVariants(motion.staggerChildren)).toEqual({
      animate: { transition: { staggerChildren: 0.04 } },
    });
  });
});

function Probe({ id }: { id: string }) {
  const v = useMotion();
  const d = pickDuration(v.fadeUp.transition) ?? -1;
  return <span data-testid={id}>{d}</span>;
}

describe("MotionRoot reduced-motion swap", () => {
  it("provides the default vocabulary when motion is allowed", () => {
    render(
      <MotionRoot forceReducedMotion={false}>
        <Probe id="probe-allow" />
      </MotionRoot>,
    );
    expect(screen.getByTestId("probe-allow").textContent).toBe(String(pickDuration(motion.fadeUp.transition)));
  });

  it("swaps to the fallback vocabulary when reduced motion is requested", () => {
    render(
      <MotionRoot forceReducedMotion={true}>
        <Probe id="probe-reduced" />
      </MotionRoot>,
    );
    expect(screen.getByTestId("probe-reduced").textContent).toBe("0");
  });
});
