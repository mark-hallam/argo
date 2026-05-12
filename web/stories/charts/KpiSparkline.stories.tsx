import type { Meta, StoryObj } from "@storybook/react";
import { KpiSparkline } from "@/components/ui/charts";

const meta: Meta<typeof KpiSparkline> = {
  title: "Charts/KpiSparkline",
  component: KpiSparkline,
  argTypes: {
    tone: {
      control: "select",
      options: ["success", "warning", "muted", "destructive", "info"],
    },
    height: { control: { type: "number", min: 12, max: 80 } },
    width: { control: { type: "number", min: 30, max: 300 } },
  },
  args: {
    ariaLabel: "Sparkline trend",
    height: 24,
    width: 80,
  },
};

export default meta;

type Story = StoryObj<typeof KpiSparkline>;

export const Default: Story = {
  args: {
    data: [4, 6, 5, 7, 9, 11, 8],
    tone: "success",
  },
};

export const ToneVariants: Story = {
  render: (args) => (
    <div className="flex flex-col gap-2">
      <KpiSparkline {...args} tone="success" data={[3, 5, 4, 6, 8, 9, 10]} />
      <KpiSparkline {...args} tone="warning" data={[8, 7, 7, 6, 5, 4, 3]} />
      <KpiSparkline {...args} tone="muted" data={[5, 5, 5, 5, 5, 5, 5]} />
      <KpiSparkline {...args} tone="destructive" data={[2, 5, 3, 9, 4, 12, 6]} />
      <KpiSparkline {...args} tone="info" data={[1, 2, 3, 2, 4, 3, 5]} />
    </div>
  ),
  args: { data: [] },
};

export const EmptyData: Story = {
  args: {
    data: [],
    tone: "muted",
    ariaLabel: "No data available",
  },
};

export const SinglePoint: Story = {
  args: {
    data: [42],
    tone: "info",
    ariaLabel: "One reading",
  },
};

export const AllZeros: Story = {
  args: {
    data: [0, 0, 0, 0, 0, 0, 0],
    tone: "muted",
    ariaLabel: "Zero baseline",
  },
};

export const NaNHandling: Story = {
  args: {
    data: [3, Number.NaN, 5, 6, Number.POSITIVE_INFINITY, 7, 8],
    tone: "warning",
    ariaLabel: "Series with bad readings",
  },
};

export const LargeFootprint: Story = {
  args: {
    data: Array.from({ length: 30 }, (_, i) => Math.sin(i / 2) * 10 + 20),
    tone: "info",
    width: 240,
    height: 48,
  },
};
