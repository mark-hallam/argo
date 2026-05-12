import type { Meta, StoryObj } from "@storybook/react";
import { DeltaChip } from "@/components/ui/charts";

const meta: Meta<typeof DeltaChip> = {
  title: "Charts/DeltaChip",
  component: DeltaChip,
  argTypes: {
    direction: {
      control: "select",
      options: ["up", "down", "flat"],
    },
  },
  args: {
    value: 12,
    unit: "%",
    direction: "up",
    directionMeansGood: true,
  },
};

export default meta;

type Story = StoryObj<typeof DeltaChip>;

export const UpGood: Story = {
  name: "Up = good (success)",
  args: { value: 12, unit: "%", direction: "up", directionMeansGood: true },
};

export const DownGood: Story = {
  name: "Down = good (success — e.g. response time)",
  args: { value: -3.4, unit: "s", direction: "down", directionMeansGood: false },
};

export const UpBad: Story = {
  name: "Up = bad (destructive — e.g. churn)",
  args: { value: 5, unit: "%", direction: "up", directionMeansGood: false },
};

export const Flat: Story = {
  name: "Flat (muted)",
  args: { value: 0, direction: "flat" },
};

export const Gallery: Story = {
  render: () => (
    <div className="flex flex-wrap gap-3">
      <DeltaChip value={18} unit="%" direction="up" />
      <DeltaChip value={-7} unit="%" direction="down" />
      <DeltaChip value={-2.1} unit="s" direction="down" directionMeansGood={false} />
      <DeltaChip value={4} unit="%" direction="up" directionMeansGood={false} />
      <DeltaChip value={0} direction="flat" />
    </div>
  ),
};

export const NoUnit: Story = {
  args: { value: 21, direction: "up" },
};
