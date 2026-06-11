import type { Meta, StoryObj } from "@storybook/react";
import { CapabilityGrantCard } from "@/components/skycot/worker/CapabilityGrantCard";
import type { CapabilityGrantRequest } from "@/lib/types/capability-grant";

/**
 * Connect-on-demand capability-grant card — locked `G2 · Elevated`
 * design (2026-06-11). The moment a teammate needs a connector it
 * doesn't yet have, surfaced in its conversation: one-line ask + flow
 * map + per-scope read/write toggles + Connect / Not now.
 */

const salesforce: CapabilityGrantRequest = {
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
      description: "Deal amounts, stages, next steps — Nora can update these",
      access: "write",
      grantable: ["read", "write"],
    },
  ],
};

const meta: Meta<typeof CapabilityGrantCard> = {
  title: "Skycot/Worker/CapabilityGrantCard",
  component: CapabilityGrantCard,
  args: {
    request: salesforce,
    onConnect: (scopes) => console.log("connect", scopes),
    onDismiss: () => console.log("dismiss"),
  },
  decorators: [
    (Story) => (
      <div className="max-w-2xl p-6">
        <Story />
      </div>
    ),
  ],
};

export default meta;

type Story = StoryObj<typeof CapabilityGrantCard>;

export const ReadAndWrite: Story = {};

export const Connecting: Story = {
  args: { busy: true },
};

export const SingleReadOnlyScope: Story = {
  args: {
    request: {
      connector: { id: "google-drive", name: "Google Drive" },
      teammateName: "Patrick",
      askLine: "Connect Google Drive so Patrick can read the Q3 report",
      scopes: [
        {
          id: "files",
          label: "Files",
          description: "View documents Patrick needs — read only",
          access: "read",
          grantable: ["read"],
        },
      ],
    },
  },
};
