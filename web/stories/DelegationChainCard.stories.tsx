import type { Meta, StoryObj } from "@storybook/react";
import { DelegationChainCard } from "@/components/skycot/audit/DelegationChainCard";
import type { DelegationStep } from "@/lib/api/audit-detail";

const base: DelegationStep[] = [
  {
    workerId: "019e6785-928a-7270-a766-f945b6cd7161",
    workerName: "Mariah",
    actionId: "00112233445566778899aabbccddeeff",
    role: "originator",
    ts: "2026-05-25T22:10:00Z",
  },
  {
    workerId: "019e6785-928d-78b1-a6b2-136d9ed263ce",
    workerName: "Peta",
    actionId: "11112233445566778899aabbccddeeff",
    role: "delegated_to",
    ts: "2026-05-25T22:11:00Z",
  },
  {
    workerId: "019e6785-928f-7281-bf8d-e062bc277478",
    workerName: "Nori",
    actionId: "22222233445566778899aabbccddeeff",
    role: "executed",
    ts: "2026-05-25T22:12:00Z",
  },
];

const meta: Meta<typeof DelegationChainCard> = {
  title: "Skycot/Audit/DelegationChainCard",
  component: DelegationChainCard,
  args: { chain: base },
  decorators: [
    (Story) => (
      <div className="max-w-3xl p-6">
        <Story />
      </div>
    ),
  ],
};

export default meta;

type Story = StoryObj<typeof DelegationChainCard>;

export const Empty: Story = {
  args: { chain: [] },
};

export const TwoStep: Story = {
  args: { chain: [base[0], { ...base[2], role: "executed" }] },
};

export const FiveStep: Story = {
  args: {
    chain: [
      base[0],
      base[1],
      {
        workerId: "019e6785-928e-7f20-803b-55266e526011",
        workerName: "Jamal",
        actionId: "33332233445566778899aabbccddeeff",
        role: "delegated_to",
        ts: "2026-05-25T22:12:00Z",
      },
      {
        workerId: "018f67d1-0000-7000-8000-000000000005",
        workerName: "Priya",
        actionId: "44442233445566778899aabbccddeeff",
        role: "delegated_to",
        ts: "2026-05-25T22:13:00Z",
      },
      { ...base[2], ts: "2026-05-25T22:14:00Z" },
    ],
  },
};
