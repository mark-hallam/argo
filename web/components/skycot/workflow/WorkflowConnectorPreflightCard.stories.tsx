import type { Meta, StoryObj } from "@storybook/react";
import type { TenantConnectorRow } from "@/lib/api/connectors";
import {
  WorkflowConnectorPreflightCard,
  type WorkflowConnectorPreflightCardProps,
} from "./WorkflowConnectorPreflightCard";

const meta: Meta<typeof WorkflowConnectorPreflightCard> = {
  title: "Skycot/Workflow/WorkflowConnectorPreflightCard",
  component: WorkflowConnectorPreflightCard,
};
export default meta;

type Story = StoryObj<typeof WorkflowConnectorPreflightCard>;

const REFS: WorkflowConnectorPreflightCardProps["references"] = [
  { connectorId: "zendesk", displayName: "Zendesk", stepIds: ["s1", "s2"] },
  { connectorId: "slack", displayName: "Slack", stepIds: ["s3"] },
  { connectorId: "hubspot", displayName: "HubSpot", stepIds: ["s4"] },
];

function statuses(
  spec: Record<string, TenantConnectorRow["status"]>,
): Record<string, TenantConnectorRow> {
  return Object.fromEntries(
    Object.entries(spec).map(([id, status]) => [id, { connectorId: id, status }]),
  );
}

export const AllGreen: Story = {
  args: {
    references: REFS,
    statuses: statuses({
      zendesk: "connected",
      slack: "connected",
      hubspot: "connected",
    }),
  },
};

export const OneAmber: Story = {
  args: {
    references: REFS,
    statuses: statuses({
      zendesk: "connected",
      slack: "reauth_needed",
      hubspot: "connected",
    }),
  },
};

export const AllRed: Story = {
  args: {
    references: REFS,
    statuses: statuses({
      zendesk: "not_connected",
      slack: "connect_failed",
      hubspot: "not_connected",
    }),
  },
};

export const Loading: Story = {
  args: {
    references: REFS,
    statuses: {},
    isLoading: true,
  },
};

export const NoConnectors: Story = {
  args: {
    references: [],
    statuses: {},
  },
};
