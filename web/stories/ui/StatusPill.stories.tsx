import type { Meta, StoryObj } from "@storybook/react";
import { StatusPill } from "@/components/ui/StatusPill";

const meta: Meta<typeof StatusPill> = {
  title: "UI/StatusPill",
  component: StatusPill,
  argTypes: {
    tone: {
      control: "select",
      options: ["success", "warning", "error", "info", "muted"],
    },
    size: { control: "select", options: ["sm", "md"] },
    dot: { control: "boolean" },
  },
  args: { tone: "muted", size: "md", dot: true, children: "Idle" },
};

export default meta;

type Story = StoryObj<typeof StatusPill>;

export const Success: Story = {
  args: { tone: "success", children: "Working" },
};

export const Warning: Story = {
  args: { tone: "warning", children: "Awaiting input" },
};

export const Error: Story = {
  args: { tone: "error", children: "Escalated" },
};

export const Info: Story = {
  args: { tone: "info", children: "Onboarding" },
};

export const Muted: Story = {
  args: { tone: "muted", children: "Idle" },
};

export const NoDot: Story = {
  args: { tone: "info", dot: false, children: "v1.2.0" },
};

export const SmallSize: Story = {
  args: { tone: "success", size: "sm", children: "Live" },
};

export const Tones: Story = {
  name: "Tones — gallery",
  render: () => (
    <div className="flex flex-wrap items-center gap-2">
      <StatusPill tone="success">Working</StatusPill>
      <StatusPill tone="warning">Awaiting input</StatusPill>
      <StatusPill tone="error">Escalated</StatusPill>
      <StatusPill tone="info">Onboarding</StatusPill>
      <StatusPill tone="muted">Idle</StatusPill>
    </div>
  ),
};

export const Sizes: Story = {
  name: "Sizes — sm vs md",
  render: () => (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <StatusPill tone="success" size="sm">Working</StatusPill>
        <StatusPill tone="warning" size="sm">Awaiting input</StatusPill>
        <StatusPill tone="error" size="sm">Escalated</StatusPill>
        <StatusPill tone="info" size="sm">Onboarding</StatusPill>
        <StatusPill tone="muted" size="sm">Idle</StatusPill>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <StatusPill tone="success" size="md">Working</StatusPill>
        <StatusPill tone="warning" size="md">Awaiting input</StatusPill>
        <StatusPill tone="error" size="md">Escalated</StatusPill>
        <StatusPill tone="info" size="md">Onboarding</StatusPill>
        <StatusPill tone="muted" size="md">Idle</StatusPill>
      </div>
    </div>
  ),
};

export const DotVsNoDot: Story = {
  name: "Dot — with vs without",
  render: () => (
    <div className="flex flex-wrap items-center gap-2">
      <StatusPill tone="success" dot>With dot</StatusPill>
      <StatusPill tone="success" dot={false}>No dot</StatusPill>
    </div>
  ),
};
