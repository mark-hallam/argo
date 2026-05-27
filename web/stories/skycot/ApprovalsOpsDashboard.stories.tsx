import type { Meta, StoryObj } from "@storybook/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ApprovalsOpsDashboard } from "@/components/skycot/approvals/ApprovalsOpsDashboard";
import { SAMPLE_APPROVALS, type ApprovalItem } from "@/lib/sample-approvals";

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
});

const meta: Meta<typeof ApprovalsOpsDashboard> = {
  title: "Skycot/Approvals/OpsDashboard",
  component: ApprovalsOpsDashboard,
  parameters: { layout: "fullscreen" },
  decorators: [
    (Story) => (
      <QueryClientProvider client={queryClient}>
        <div className="bg-[var(--bg-1)] p-6">
          <Story />
        </div>
      </QueryClientProvider>
    ),
  ],
};

export default meta;

type Story = StoryObj<typeof ApprovalsOpsDashboard>;

export const Seeded: Story = {
  name: "Seeded — 5 pending + 3 resolved",
  args: { items: SAMPLE_APPROVALS },
};

const LONG_TOKEN =
  "reproductivity-of-the-customer-segment-across-the-same-API-surface";

const longWordRow: ApprovalItem = {
  id: "ap_long_word",
  worker: { name: "Peta" },
  verb: "wants approval",
  detail: `Throttle ${LONG_TOKEN} for the day so we don't burn quota.`,
  consequenceIfYes: "yes",
  consequenceIfNo: "no",
  channel: "slack",
  severity: "p0",
  dollarAtRisk: 48_000,
  confidence: 0.82,
  receivedAt: new Date(Date.now() - 60 * 60_000).toISOString(),
};

export const HighTierLongWordCard: Story = {
  name: "HighTier — long-word card (scene-6 F4 regression)",
  args: {
    items: [longWordRow, ...SAMPLE_APPROVALS.filter((it) => it.severity !== "p0")],
  },
};
