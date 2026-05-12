import type { Meta, StoryObj } from "@storybook/react";
import { ProgressBar } from "@/components/ui/charts";

const meta: Meta<typeof ProgressBar> = {
  title: "Charts/ProgressBar",
  component: ProgressBar,
  argTypes: {
    value: { control: { type: "range", min: 0, max: 100, step: 1 } },
    height: { control: { type: "number", min: 2, max: 24 } },
  },
  args: {
    value: 50,
    height: 6,
    ariaLabel: "Progress",
  },
  decorators: [
    (Story) => (
      <div style={{ width: 280 }}>
        <Story />
      </div>
    ),
  ],
};

export default meta;

type Story = StoryObj<typeof ProgressBar>;

export const Default: Story = {
  args: { value: 60 },
};

export const WithThresholds: Story = {
  args: {
    value: 72,
    thresholds: { warning: 60, destructive: 80 },
    ariaLabel: "Disk usage",
  },
};

export const Empty: Story = {
  args: { value: 0, ariaLabel: "Nothing started" },
};

export const Full: Story = {
  args: { value: 100, ariaLabel: "Completed" },
};

export const ToneEscalation: Story = {
  render: (args) => (
    <div className="flex flex-col gap-3">
      <ProgressBar {...args} value={30} ariaLabel="30% — muted" />
      <ProgressBar {...args} value={65} ariaLabel="65% — warning" />
      <ProgressBar {...args} value={92} ariaLabel="92% — destructive" />
    </div>
  ),
  args: {
    value: 0,
    thresholds: { warning: 60, destructive: 80 },
  },
};

export const Clamped: Story = {
  args: {
    value: 250,
    ariaLabel: "Out-of-range clamps to 100",
  },
};

export const NaNFallback: Story = {
  args: {
    value: Number.NaN,
    ariaLabel: "NaN renders at 0",
  },
};
