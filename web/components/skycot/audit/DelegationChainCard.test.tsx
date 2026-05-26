import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { DelegationChainCard } from "@/components/skycot/audit/DelegationChainCard";
import type { DelegationStep } from "@/lib/api/audit-detail";

vi.mock("next/link", () => ({
  __esModule: true,
  default: ({ children, ...props }: { children: React.ReactNode }) => (
    <a {...props}>{children}</a>
  ),
}));

const CHAIN: DelegationStep[] = [
  {
    workerId: "wkr_mariah",
    workerName: "Mariah",
    actionId: "00112233445566778899aabbccddeeff",
    role: "originator",
    ts: "2026-05-25T22:10:00Z",
  },
  {
    workerId: "wkr_peta",
    workerName: "Peta",
    actionId: "11112233445566778899aabbccddeeff",
    role: "delegated_to",
    ts: "2026-05-25T22:11:00Z",
  },
  {
    workerId: "wkr_nori",
    workerName: "Nori",
    actionId: "22222233445566778899aabbccddeeff",
    role: "executed",
    ts: "2026-05-25T22:12:00Z",
  },
];

describe("DelegationChainCard", () => {
  it("renders nothing for an empty chain", () => {
    const { container } = render(<DelegationChainCard chain={[]} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("renders one row per delegation step and arrows between them", () => {
    render(<DelegationChainCard chain={CHAIN} />);
    const card = screen.getByTestId("audit-delegation-chain");
    expect(within(card).getAllByTestId(/delegation-chain-step-/)).toHaveLength(3);
    expect(within(card).getAllByTestId(/delegation-chain-arrow-/)).toHaveLength(2);
    expect(card).toHaveTextContent("Originator");
    expect(card).toHaveTextContent("Delegated");
    expect(card).toHaveTextContent("Executed");
  });

  it("links worker chips to worker overview pages", () => {
    render(<DelegationChainCard chain={CHAIN} />);
    expect(screen.getByTestId("delegation-chain-worker-0")).toHaveAttribute(
      "href",
      "/workers/wkr_mariah/overview",
    );
    expect(screen.getByTestId("delegation-chain-worker-2")).toHaveAttribute(
      "href",
      "/workers/wkr_nori/overview",
    );
  });
});
