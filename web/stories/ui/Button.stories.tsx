import type { Meta, StoryObj } from "@storybook/react";
import { ArrowRight, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/Button";

const meta: Meta<typeof Button> = {
  title: "UI/Button",
  component: Button,
  argTypes: {
    variant: {
      control: "select",
      options: ["primary", "secondary", "ghost", "destructive", "link"],
    },
    size: {
      control: "select",
      options: ["sm", "md", "lg", "icon"],
    },
    loading: { control: "boolean" },
    disabled: { control: "boolean" },
    asChild: { control: "boolean" },
  },
  args: {
    children: "Hire teammate",
    variant: "primary",
    size: "md",
    loading: false,
    disabled: false,
  },
};

export default meta;

type Story = StoryObj<typeof Button>;

export const Primary: Story = {
  args: { variant: "primary", children: "Hire teammate" },
};

export const Secondary: Story = {
  args: { variant: "secondary", children: "Cancel" },
};

export const Ghost: Story = {
  args: { variant: "ghost", children: "More" },
};

export const Destructive: Story = {
  args: { variant: "destructive", children: "Delete worker" },
};

export const Link: Story = {
  args: { variant: "link", children: "View profile" },
};

export const Disabled: Story = {
  args: { variant: "primary", children: "Hire teammate", disabled: true },
};

export const Loading: Story = {
  args: { variant: "primary", children: "Hiring…", loading: true },
};

export const WithIcon: Story = {
  args: {
    variant: "primary",
    children: (
      <>
        Continue
        <ArrowRight />
      </>
    ),
  },
};

export const IconOnly: Story = {
  args: {
    size: "icon",
    variant: "ghost",
    "aria-label": "Delete",
    children: <Trash2 />,
  },
};

export const Sizes: Story = {
  name: "Sizes — sm / md / lg",
  render: (args) => (
    <div className="flex items-center gap-3">
      <Button {...args} size="sm">
        Small
      </Button>
      <Button {...args} size="md">
        Medium
      </Button>
      <Button {...args} size="lg">
        Large
      </Button>
    </div>
  ),
  args: { variant: "primary", children: undefined },
};

export const Variants: Story = {
  name: "Variants — gallery",
  render: () => (
    <div className="flex flex-wrap items-center gap-3">
      <Button variant="primary">Primary</Button>
      <Button variant="secondary">Secondary</Button>
      <Button variant="ghost">Ghost</Button>
      <Button variant="destructive">Destructive</Button>
      <Button variant="link">Link</Button>
    </div>
  ),
};

export const States: Story = {
  name: "States — default / disabled / loading",
  render: () => (
    <div className="flex flex-wrap items-center gap-3">
      <Button>Default</Button>
      <Button disabled>Disabled</Button>
      <Button loading>Loading</Button>
    </div>
  ),
};

export const AsChildAnchor: Story = {
  name: "asChild — anchor composition",
  args: {
    asChild: true,
    variant: "secondary",
    children: <a href="#profile">Open profile</a>,
  },
};
