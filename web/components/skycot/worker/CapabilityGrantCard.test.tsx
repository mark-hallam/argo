import { render, screen, fireEvent, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CapabilityGrantCard } from "./CapabilityGrantCard";
import type { CapabilityGrantRequest } from "@/lib/types/capability-grant";

/**
 * CapabilityGrantCard — locked `G2 · Elevated` connect-on-demand card.
 *
 * Presentational contract:
 *   - renders the one-line ask + Audiowide eyebrow
 *   - one read/write toggle group per scope
 *   - one flow-map edge per scope, keyed on the current toggle access
 *   - toggling write→read swaps the edge class + the row access
 *   - onConnect fires with the selected `{id, access}[]`
 *   - onDismiss fires
 */

const REQUEST: CapabilityGrantRequest = {
  connector: { id: "salesforce", name: "Salesforce" },
  teammateName: "Nora",
  askLine: "Connect Salesforce so Nora can read your pipeline",
  scopes: [
    {
      id: "contacts",
      label: "Contacts",
      description: "Names, emails, owners — Nora only looks",
      access: "read",
      grantable: ["read", "write"],
    },
    {
      id: "opportunities",
      label: "Opportunities",
      description: "Deal amounts, stages, next steps",
      access: "write",
      grantable: ["read", "write"],
    },
  ],
};

function renderCard(overrides: Partial<React.ComponentProps<typeof CapabilityGrantCard>> = {}) {
  const onConnect = vi.fn();
  const onDismiss = vi.fn();
  render(
    <CapabilityGrantCard
      request={REQUEST}
      onConnect={onConnect}
      onDismiss={onDismiss}
      {...overrides}
    />,
  );
  return { onConnect, onDismiss };
}

describe("CapabilityGrantCard", () => {
  it("renders the one-line ask, eyebrow, and a row per scope", () => {
    renderCard();
    expect(
      screen.getByText("Connect Salesforce so Nora can read your pipeline"),
    ).toBeInTheDocument();
    expect(screen.getByText(/Connect on demand/i)).toBeInTheDocument();
    expect(screen.getByTestId("scope-row-contacts")).toBeInTheDocument();
    expect(screen.getByTestId("scope-row-opportunities")).toBeInTheDocument();
  });

  it("renders one read/write toggle group per scope", () => {
    renderCard();
    for (const id of ["contacts", "opportunities"]) {
      expect(screen.getByTestId(`scope-toggle-read-${id}`)).toBeInTheDocument();
      expect(screen.getByTestId(`scope-toggle-write-${id}`)).toBeInTheDocument();
    }
  });

  it("renders the flow map with one edge per scope, keyed on access", () => {
    renderCard();
    expect(screen.getByTestId("capability-grant-flow-map")).toBeInTheDocument();
    const contactsEdge = screen.getByTestId("flow-edge-contacts");
    const oppsEdge = screen.getByTestId("flow-edge-opportunities");
    expect(contactsEdge.getAttribute("data-access")).toBe("read");
    expect(contactsEdge.getAttribute("class")).toContain("cg-flow-edge--read");
    expect(oppsEdge.getAttribute("data-access")).toBe("write");
    expect(oppsEdge.getAttribute("class")).toContain("cg-flow-edge--write");
    expect(oppsEdge.getAttribute("class")).toContain("cg-flow-edge--live");
  });

  it("toggling write→read live-updates the row + the flow-map edge", () => {
    renderCard();
    // Opportunities starts as write.
    expect(screen.getByTestId("flow-edge-opportunities").getAttribute("data-access")).toBe("write");
    fireEvent.click(screen.getByTestId("scope-toggle-read-opportunities"));
    expect(screen.getByTestId("scope-row-opportunities").getAttribute("data-access")).toBe("read");
    const edge = screen.getByTestId("flow-edge-opportunities");
    expect(edge.getAttribute("data-access")).toBe("read");
    expect(edge.getAttribute("class")).toContain("cg-flow-edge--read");
    expect(edge.getAttribute("class")).not.toContain("cg-flow-edge--live");
  });

  it("onConnect fires with the selected per-scope access set", () => {
    const { onConnect } = renderCard();
    // Flip Contacts to write before connecting.
    fireEvent.click(screen.getByTestId("scope-toggle-write-contacts"));
    fireEvent.click(screen.getByTestId("capability-grant-connect"));
    expect(onConnect).toHaveBeenCalledTimes(1);
    expect(onConnect).toHaveBeenCalledWith([
      { id: "contacts", access: "write" },
      { id: "opportunities", access: "write" },
    ]);
  });

  it("onDismiss fires on Not now", () => {
    const { onDismiss } = renderCard();
    fireEvent.click(screen.getByTestId("capability-grant-dismiss"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("busy disables the actions + toggles and shows progress copy", () => {
    const { onConnect } = renderCard({ busy: true });
    const connect = screen.getByTestId("capability-grant-connect");
    expect(connect).toBeDisabled();
    expect(connect.textContent).toMatch(/Connecting/i);
    expect(screen.getByTestId("capability-grant-dismiss")).toBeDisabled();
    const row = screen.getByTestId("scope-row-contacts");
    expect(within(row).getByTestId("scope-toggle-read-contacts")).toBeDisabled();
    fireEvent.click(connect);
    expect(onConnect).not.toHaveBeenCalled();
  });

  it("disables a toggle the scope can't grant", () => {
    render(
      <CapabilityGrantCard
        request={{
          ...REQUEST,
          scopes: [
            {
              id: "readonly",
              label: "Read-only feed",
              description: "View only",
              access: "read",
              grantable: ["read"],
            },
          ],
        }}
        onConnect={vi.fn()}
        onDismiss={vi.fn()}
      />,
    );
    expect(screen.getByTestId("scope-toggle-write-readonly")).toBeDisabled();
    expect(screen.getByTestId("scope-toggle-read-readonly")).not.toBeDisabled();
  });
});
