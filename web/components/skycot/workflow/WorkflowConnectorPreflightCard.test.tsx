import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { TenantConnectorRow } from "@/lib/api/connectors";
import type { ConnectorReference } from "@/lib/workflow/extract-connectors";
import {
  WorkflowConnectorPreflightCard,
  allConnectorsHealthy,
  metaForStatus,
} from "./WorkflowConnectorPreflightCard";
import { extractConnectorReferences } from "@/lib/workflow/extract-connectors";

const CATALOG = [
  { id: "zendesk", name: "Zendesk" },
  { id: "slack", name: "Slack" },
  { id: "hubspot", name: "HubSpot" },
  { id: "google-meet", name: "Google Meet" },
] as const;

function row(
  connectorId: string,
  status: TenantConnectorRow["status"],
): TenantConnectorRow {
  return { connectorId, status };
}

const ZENDESK: ConnectorReference = {
  connectorId: "zendesk",
  displayName: "Zendesk",
  stepIds: ["step-1", "step-2"],
};

const SLACK: ConnectorReference = {
  connectorId: "slack",
  displayName: "Slack",
  stepIds: ["step-3"],
};

const HUBSPOT: ConnectorReference = {
  connectorId: "hubspot",
  displayName: "HubSpot",
  stepIds: ["step-4"],
};

describe("WorkflowConnectorPreflightCard", () => {
  it("renders no card when references list is empty (acceptance 5)", () => {
    const { container } = render(
      <WorkflowConnectorPreflightCard references={[]} statuses={{}} />,
    );
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByTestId("wfd-connector-preflight")).toBeNull();
  });

  it("renders the all-green case with connected status for every connector", () => {
    render(
      <WorkflowConnectorPreflightCard
        references={[ZENDESK, SLACK]}
        statuses={{
          zendesk: row("zendesk", "connected"),
          slack: row("slack", "connected"),
        }}
      />,
    );
    const card = screen.getByTestId("wfd-connector-preflight");
    expect(card).toBeInTheDocument();

    const zendeskRow = screen.getByTestId(
      "wfd-connector-preflight-row-zendesk",
    );
    expect(zendeskRow.getAttribute("data-health")).toBe("ok");
    expect(within(zendeskRow).getByText("Connected")).toBeInTheDocument();
    expect(within(zendeskRow).getByText("Used in 2 steps")).toBeInTheDocument();

    const slackRow = screen.getByTestId("wfd-connector-preflight-row-slack");
    expect(slackRow.getAttribute("data-health")).toBe("ok");
    expect(within(slackRow).getByText("Used in 1 step")).toBeInTheDocument();
  });

  it("renders the one-amber case for a reauth-needed connector", () => {
    render(
      <WorkflowConnectorPreflightCard
        references={[ZENDESK, SLACK]}
        statuses={{
          zendesk: row("zendesk", "connected"),
          slack: row("slack", "reauth_needed"),
        }}
      />,
    );
    const slackRow = screen.getByTestId("wfd-connector-preflight-row-slack");
    expect(slackRow.getAttribute("data-health")).toBe("warn");
    expect(within(slackRow).getByText("Needs reconnect")).toBeInTheDocument();
  });

  it("renders the all-red case when nothing is connected", () => {
    render(
      <WorkflowConnectorPreflightCard
        references={[ZENDESK, HUBSPOT]}
        // no statuses → defaults to not_connected
        statuses={{}}
      />,
    );
    const zendeskRow = screen.getByTestId(
      "wfd-connector-preflight-row-zendesk",
    );
    expect(zendeskRow.getAttribute("data-health")).toBe("blocked");
    expect(within(zendeskRow).getByText("Not connected")).toBeInTheDocument();

    const hubspotRow = screen.getByTestId(
      "wfd-connector-preflight-row-hubspot",
    );
    expect(hubspotRow.getAttribute("data-health")).toBe("blocked");
  });

  it("renders a skeleton per row while loading and no status is available", () => {
    render(
      <WorkflowConnectorPreflightCard
        references={[ZENDESK]}
        statuses={{}}
        isLoading
      />,
    );
    expect(
      screen.getByTestId("wfd-connector-preflight-row-zendesk-skeleton"),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId("wfd-connector-preflight-row-zendesk-link"),
    ).toBeNull();
  });

  it("links each row through to /settings/connectors/<id> by default", () => {
    render(
      <WorkflowConnectorPreflightCard
        references={[ZENDESK]}
        statuses={{ zendesk: row("zendesk", "not_connected") }}
      />,
    );
    const link = screen.getByTestId(
      "wfd-connector-preflight-row-zendesk-link",
    );
    expect(link.getAttribute("href")).toBe("/settings/connectors/zendesk");
  });
});

describe("metaForStatus", () => {
  it("maps each ConnectorStatus to a stable health bucket", () => {
    expect(metaForStatus("connected").health).toBe("ok");
    expect(metaForStatus("reauth_needed").health).toBe("warn");
    expect(metaForStatus("not_connected").health).toBe("blocked");
    expect(metaForStatus("connect_failed").health).toBe("blocked");
    expect(metaForStatus("connecting").health).toBe("pending");
    expect(metaForStatus(undefined).health).toBe("blocked");
  });
});

describe("allConnectorsHealthy", () => {
  it("returns true when every reference is connected", () => {
    expect(
      allConnectorsHealthy([ZENDESK, SLACK], {
        zendesk: row("zendesk", "connected"),
        slack: row("slack", "connected"),
      }),
    ).toBe(true);
  });

  it("returns false when any reference is unhealthy", () => {
    expect(
      allConnectorsHealthy([ZENDESK, SLACK], {
        zendesk: row("zendesk", "connected"),
        slack: row("slack", "reauth_needed"),
      }),
    ).toBe(false);
  });

  it("returns true vacuously for an empty references list", () => {
    expect(allConnectorsHealthy([], {})).toBe(true);
  });
});

describe("extractConnectorReferences (heuristic match)", () => {
  it("matches connector ids and display names in step text", () => {
    const refs = extractConnectorReferences(
      [
        {
          id: "s1",
          kind: "skill_invoke",
          name: "Fetch Zendesk thread",
          description: "Pull the latest reply for the ticket.",
        },
        {
          id: "s2",
          kind: "notify",
          name: "Post digest to Slack #standup",
          description: "Threaded message in #standup.",
        },
        {
          id: "s3",
          kind: "decision",
          name: "Branch on intent",
          description: "Self-deflect or escalate.",
        },
      ],
      CATALOG,
    );
    expect(refs.map((r) => r.connectorId).sort()).toEqual(["slack", "zendesk"]);
    const zendesk = refs.find((r) => r.connectorId === "zendesk")!;
    expect(zendesk.stepIds).toEqual(["s1"]);
  });

  it("deduplicates a connector referenced by multiple steps", () => {
    const refs = extractConnectorReferences(
      [
        { id: "s1", name: "Validate via HubSpot" },
        { id: "s2", name: "Update HubSpot deal" },
      ],
      CATALOG,
    );
    expect(refs).toHaveLength(1);
    expect(refs[0]?.stepIds).toEqual(["s1", "s2"]);
  });

  it("matches a hyphenated connector id (google-meet) inside step text", () => {
    const refs = extractConnectorReferences(
      [{ id: "s1", description: "Book a Google Meet for the candidate." }],
      CATALOG,
    );
    expect(refs.map((r) => r.connectorId)).toEqual(["google-meet"]);
  });

  it("respects an explicit server / app / tool field on future step shapes", () => {
    const refs = extractConnectorReferences(
      [
        {
          id: "s1",
          kind: "mcp_invoke",
          name: "fetch_messages",
          description: "loads recent threads",
          server: "slack",
        },
      ],
      CATALOG,
    );
    expect(refs.map((r) => r.connectorId)).toEqual(["slack"]);
  });

  it("returns an empty list when no catalog entry matches", () => {
    const refs = extractConnectorReferences(
      [
        { id: "s1", name: "Classify severity", description: "P0..P3 tagger." },
        { id: "s2", name: "Decide branch", description: "VIP escalation." },
      ],
      CATALOG,
    );
    expect(refs).toEqual([]);
  });
});
