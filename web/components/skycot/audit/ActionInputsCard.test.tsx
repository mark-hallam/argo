import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ActionInputsCard } from "@/components/skycot/audit/ActionInputsCard";
import { POLICY_SHOW_ALL } from "@/lib/redaction/redact";
import type { AuditDetailInputs, AuditTriggerKind } from "@/lib/types/audit-detail";

vi.mock("next/link", () => ({
  __esModule: true,
  default: ({ children, ...props }: { children: React.ReactNode }) => (
    <a {...props}>{children}</a>
  ),
}));

function inputs(overrides: Partial<AuditDetailInputs> = {}): AuditDetailInputs {
  return {
    triggerKind: "message",
    triggerSummary: "Inbound customer email",
    ...overrides,
  };
}

describe("ActionInputsCard", () => {
  it("renders the branded label for a known trigger kind", () => {
    render(<ActionInputsCard inputs={inputs({ triggerKind: "cron" })} policy={POLICY_SHOW_ALL} />);
    const card = screen.getByTestId("audit-inputs-card");
    expect(card).toHaveAttribute("data-trigger-kind", "cron");
    expect(card).toHaveTextContent("Cron");
    expect(screen.getByTestId("audit-inputs-summary")).toHaveTextContent("Inbound customer email");
  });

  // The gateway reads `trigger_kind` straight from the (free-form) audit
  // event payload and the SPA casts it through with `as AuditTriggerKind`
  // — neither side clamps it to the union. An off-union kind must degrade
  // to a neutral icon + humanized label, never render `<undefined />` and
  // crash the audit drill-in. (Mirrors ChannelChip / NotificationRow.)
  it("renders without throwing for a trigger kind outside the union", () => {
    // `webhook` is a real backend trigger the SPA union doesn't enumerate;
    // simulate the gateway pass-through with the same `as` cast the
    // normalizer uses.
    const offUnion = "webhook" as AuditTriggerKind;
    expect(() =>
      render(<ActionInputsCard inputs={inputs({ triggerKind: offUnion })} policy={POLICY_SHOW_ALL} />),
    ).not.toThrow();

    const card = screen.getByTestId("audit-inputs-card");
    // Degrades to a humanized label (not blank, not a crash).
    expect(card).toHaveTextContent("Webhook");
    // The fallback icon renders as a real SVG element, not `<undefined />`.
    expect(card.querySelector("svg")).not.toBeNull();
  });
});
