import type { Meta, StoryObj } from "@storybook/react";
import { MiniBar } from "@/components/ui/charts";

const meta: Meta<typeof MiniBar> = {
  title: "Charts/MiniBar",
  component: MiniBar,
  argTypes: {
    tone: {
      control: "select",
      options: ["success", "warning", "muted", "destructive", "info"],
    },
    height: { control: { type: "number", min: 12, max: 80 } },
    width: { control: { type: "number", min: 16, max: 200 } },
  },
  args: {
    ariaLabel: "Bar chart",
    height: 24,
    width: 32,
  },
};

export default meta;

type Story = StoryObj<typeof MiniBar>;

export const Default: Story = {
  args: {
    data: [1, 3, 8, 5, 6, 2, 4],
    tone: "success",
  },
};

export const ToneVariants: Story = {
  render: (args) => (
    <div className="flex items-center gap-3">
      <MiniBar {...args} tone="success" />
      <MiniBar {...args} tone="warning" />
      <MiniBar {...args} tone="muted" />
      <MiniBar {...args} tone="destructive" />
      <MiniBar {...args} tone="info" />
    </div>
  ),
  args: { data: [2, 5, 3, 7, 4, 6, 8] },
};

export const EmptyData: Story = {
  args: { data: [], tone: "muted", ariaLabel: "No data" },
};

export const SinglePoint: Story = {
  args: { data: [42], tone: "info", ariaLabel: "One day" },
};

export const AllEqual: Story = {
  args: { data: [5, 5, 5, 5, 5], tone: "muted", ariaLabel: "Steady state" },
};

export const AllZeros: Story = {
  args: {
    data: [0, 0, 0, 0, 0, 0, 0],
    tone: "muted",
    ariaLabel: "Zero baseline (min-height visible)",
  },
};

export const Wide: Story = {
  args: {
    data: [3, 7, 2, 9, 4, 6, 8, 5, 10, 6, 4, 7],
    tone: "info",
    width: 120,
    height: 40,
  },
};
