import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  DeltaChip,
  KpiSparkline,
  MiniBar,
  ProgressBar,
  normalize,
  thresholdTone,
} from "./index";

describe("normalize", () => {
  it("maps a strictly-increasing series into [0, 1] endpoints", () => {
    expect(normalize([0, 5, 10])).toEqual([0, 0.5, 1]);
  });

  it("renders a constant series at the mid-band so it draws flat", () => {
    expect(normalize([7, 7, 7])).toEqual([0.5, 0.5, 0.5]);
  });

  it("replaces non-finite points with mid-band rather than crashing", () => {
    const out = normalize([0, Number.NaN, 10]);
    expect(out[0]).toBe(0);
    expect(out[1]).toBe(0.5);
    expect(out[2]).toBe(1);
  });
});

describe("thresholdTone", () => {
  it("returns muted below the warning cutoff", () => {
    expect(thresholdTone(40, { warning: 60, destructive: 80 })).toBe("muted");
  });

  it("returns warning in the [warning, destructive) band", () => {
    expect(thresholdTone(60, { warning: 60, destructive: 80 })).toBe("warning");
    expect(thresholdTone(79, { warning: 60, destructive: 80 })).toBe("warning");
  });

  it("returns destructive at-or-above the destructive cutoff", () => {
    expect(thresholdTone(80, { warning: 60, destructive: 80 })).toBe(
      "destructive",
    );
  });
});

describe("KpiSparkline", () => {
  it("renders an SVG path and the latest-point marker for normal data", () => {
    const { container } = render(
      <KpiSparkline data={[1, 2, 3, 4, 5]} tone="success" ariaLabel="Weekly trend" />,
    );
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
    expect(svg).toHaveAttribute("aria-label", "Weekly trend");
    expect(container.querySelector("path")).not.toBeNull();
    expect(container.querySelector("circle")).not.toBeNull();
    const wrap = screen.getByTestId("kpi-sparkline");
    expect(wrap.getAttribute("data-tone")).toBe("success");
  });

  it("renders a horizontal line and no marker for an empty series", () => {
    const { container } = render(<KpiSparkline data={[]} />);
    const path = container.querySelector("path");
    expect(path?.getAttribute("d")).toMatch(/^M0,/);
    expect(container.querySelector("circle")).toBeNull();
  });

  it("uses the destructive stroke class when tone=destructive", () => {
    const { container } = render(
      <KpiSparkline data={[3, 1, 2]} tone="destructive" />,
    );
    const path = container.querySelector("path");
    expect(path?.getAttribute("class") ?? "").toContain(
      "stroke-[var(--destructive)]",
    );
  });
});

describe("ProgressBar", () => {
  it("renders a 0–100 progressbar with a fill width matching the value", () => {
    render(<ProgressBar value={42} ariaLabel="Budget used" />);
    const bar = screen.getByRole("progressbar", { name: "Budget used" });
    expect(bar).toHaveAttribute("aria-valuenow", "42");
    expect(bar).toHaveAttribute("aria-valuemin", "0");
    expect(bar).toHaveAttribute("aria-valuemax", "100");
    const fill = screen.getByTestId("progress-bar-fill");
    expect(fill.getAttribute("style") ?? "").toContain("width: 42%");
  });

  it("escalates tone past the destructive threshold", () => {
    render(
      <ProgressBar
        value={92}
        thresholds={{ warning: 60, destructive: 80 }}
        ariaLabel="Disk usage"
      />,
    );
    const bar = screen.getByRole("progressbar", { name: "Disk usage" });
    expect(bar.getAttribute("data-tone")).toBe("destructive");
  });

  it("clamps out-of-range values and treats NaN as zero", () => {
    render(<ProgressBar value={Number.NaN} ariaLabel="NaN test" />);
    const fill = screen.getByTestId("progress-bar-fill");
    expect(fill.getAttribute("style") ?? "").toContain("width: 0%");
  });
});

describe("DeltaChip", () => {
  it("up + directionMeansGood=true renders the success tone with arrow + sign", () => {
    render(<DeltaChip value={12} unit="%" direction="up" />);
    const chip = screen.getByTestId("delta-chip");
    expect(chip.getAttribute("data-tone")).toBe("success");
    expect(chip).toHaveTextContent("+12%");
  });

  it("up + directionMeansGood=false renders the destructive tone", () => {
    render(
      <DeltaChip
        value={3.4}
        unit="s"
        direction="up"
        directionMeansGood={false}
      />,
    );
    expect(screen.getByTestId("delta-chip").getAttribute("data-tone")).toBe(
      "destructive",
    );
  });

  it("direction=flat renders the muted tone regardless of meaning", () => {
    render(<DeltaChip value={0} direction="flat" />);
    const chip = screen.getByTestId("delta-chip");
    expect(chip.getAttribute("data-tone")).toBe("muted");
    expect(chip).toHaveTextContent("0");
  });
});

describe("MiniBar", () => {
  it("renders one <rect> per data point with the requested tone", () => {
    const { container } = render(
      <MiniBar data={[1, 3, 2, 5, 4]} tone="info" ariaLabel="Last 5 days" />,
    );
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("aria-label", "Last 5 days");
    const rects = container.querySelectorAll("rect");
    expect(rects.length).toBe(5);
    expect(rects[0].getAttribute("class") ?? "").toContain(
      "fill-[var(--info)]",
    );
  });

  it("renders no bars for empty data but still emits an accessible SVG", () => {
    const { container } = render(
      <MiniBar data={[]} ariaLabel="No data" />,
    );
    expect(container.querySelectorAll("rect").length).toBe(0);
    expect(container.querySelector("svg")).toHaveAttribute(
      "aria-label",
      "No data",
    );
  });

  it("enforces a 1px minimum bar height even for the smallest data point", () => {
    const { container } = render(<MiniBar data={[0, 0, 10]} />);
    const rects = container.querySelectorAll("rect");
    expect(rects.length).toBe(3);
    rects.forEach((r) => {
      const h = parseFloat(r.getAttribute("height") ?? "0");
      expect(h).toBeGreaterThanOrEqual(1);
    });
  });
});
