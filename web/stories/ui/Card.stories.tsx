import type { Meta, StoryObj } from "@storybook/react";
import {
  Card,
  CardBody,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { StatusPill } from "@/components/ui/StatusPill";

const meta: Meta<typeof Card> = {
  title: "UI/Card",
  component: Card,
  argTypes: {
    hoverable: { control: "boolean" },
    selected: { control: "boolean" },
  },
  args: { hoverable: false, selected: false },
};

export default meta;

type Story = StoryObj<typeof Card>;

export const Basic: Story = {
  render: (args) => (
    <Card {...args} className="max-w-md">
      <CardBody>
        A bare card surface. Use composition slots (header / body / footer)
        for richer content.
      </CardBody>
    </Card>
  ),
};

export const WithHeaderAndBody: Story = {
  render: (args) => (
    <Card {...args} className="max-w-md">
      <CardHeader>
        <CardTitle>Mariah — Receptionist L3</CardTitle>
        <CardDescription>
          Handles inbound calls, books appointments, escalates refunds.
        </CardDescription>
      </CardHeader>
      <CardBody>
        Completed 142 calls this week. Average handle time 2m 18s.
      </CardBody>
    </Card>
  ),
};

export const WithFooterActions: Story = {
  render: (args) => (
    <Card {...args} className="max-w-md">
      <CardHeader>
        <CardTitle>Approve refund?</CardTitle>
        <CardDescription>
          $124.50 — order #A2103 — customer cited delayed shipment.
        </CardDescription>
      </CardHeader>
      <CardBody>
        Confidence 84%. Worker recommends approval based on shipping
        carrier delay confirmed at 09:14 PT.
      </CardBody>
      <CardFooter>
        <Button variant="primary">Approve</Button>
        <Button variant="secondary">Send back</Button>
      </CardFooter>
    </Card>
  ),
};

export const Hoverable: Story = {
  args: { hoverable: true },
  render: (args) => (
    <Card {...args} className="max-w-md">
      <CardHeader>
        <CardTitle>Open worker profile</CardTitle>
        <CardDescription>Hover for elevation, click to navigate.</CardDescription>
      </CardHeader>
    </Card>
  ),
};

export const Selected: Story = {
  args: { selected: true },
  render: (args) => (
    <Card {...args} className="max-w-md">
      <CardHeader>
        <CardTitle>Mariah is selected</CardTitle>
        <CardDescription>
          Selected state shows the focus ring without elevation.
        </CardDescription>
      </CardHeader>
    </Card>
  ),
};

export const DenseRow: Story = {
  name: "Dense — single-line list row",
  render: () => (
    <div className="flex max-w-2xl flex-col gap-2">
      {[
        { name: "Mariah", role: "Receptionist L3", status: "success" as const, msg: "Working" },
        { name: "Peta", role: "Project Coordinator L2", status: "warning" as const, msg: "Awaiting input" },
        { name: "Patrick", role: "Status Reporter L3", status: "muted" as const, msg: "Idle" },
      ].map((w) => (
        <Card key={w.name} hoverable className="flex items-center justify-between px-4 py-3">
          <div className="flex flex-col">
            <span className="text-sm font-medium">{w.name}</span>
            <span className="text-xs text-[var(--muted-foreground)]">{w.role}</span>
          </div>
          <StatusPill tone={w.status}>{w.msg}</StatusPill>
        </Card>
      ))}
    </div>
  ),
};

export const SelectionGrid: Story = {
  name: "Selection — grid with one selected",
  render: () => (
    <div className="grid max-w-3xl grid-cols-3 gap-3">
      {["Mariah", "Peta", "Patrick"].map((name, i) => (
        <Card key={name} hoverable selected={i === 1}>
          <CardHeader>
            <CardTitle>{name}</CardTitle>
            <CardDescription>Click to drill in.</CardDescription>
          </CardHeader>
        </Card>
      ))}
    </div>
  ),
};
