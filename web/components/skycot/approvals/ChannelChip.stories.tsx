import type { Meta, StoryObj } from "@storybook/react";
import { ChannelChip } from "@/components/skycot/approvals/ChannelChip";
import type { ChannelKind } from "@/lib/sample-approvals";

const CHANNELS: ChannelKind[] = [
  "slack",
  "imessage",
  "sms",
  "push",
  "web",
  "voice",
  "whatsapp",
];

const meta: Meta<typeof ChannelChip> = {
  title: "Skycot/Approvals/ChannelChip",
  component: ChannelChip,
  argTypes: {
    channel: { control: "select", options: CHANNELS },
    size: { control: "radio", options: ["sm", "md"] },
  },
  args: { channel: "slack", size: "sm" },
};

export default meta;

type Story = StoryObj<typeof ChannelChip>;

export const Default: Story = {};

export const AllChannels: Story = {
  render: () => (
    <div className="flex flex-wrap items-center gap-2">
      {CHANNELS.map((channel) => (
        <ChannelChip key={channel} channel={channel} />
      ))}
    </div>
  ),
};

export const Medium: Story = {
  render: () => (
    <div className="flex flex-wrap items-center gap-2">
      {CHANNELS.map((channel) => (
        <ChannelChip key={channel} channel={channel} size="md" />
      ))}
    </div>
  ),
};
