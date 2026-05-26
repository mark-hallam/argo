import type { Meta, StoryObj } from "@storybook/react";
import type { WorkflowSubstrate } from "@/lib/api/workflows";
import { SubstrateChip, type SubstrateStatus } from "@/components/skycot/workflow/SubstrateChip";

const meta: Meta<typeof SubstrateChip> = {
  title: "Skycot/Workflow/SubstrateChip",
  component: SubstrateChip,
  argTypes: {
    substrate: {
      control: "select",
      options: ["macos", "linux", "windows", "e2b", "browser", "headless"],
    },
    status: {
      control: "select",
      options: ["idle", "running", "completed", "failed"],
    },
    compact: { control: "boolean" },
  },
  args: { substrate: "macos", status: "idle", compact: false },
};

export default meta;

type Story = StoryObj<typeof SubstrateChip>;

const SUBSTRATES: WorkflowSubstrate[] = [
  "macos",
  "linux",
  "windows",
  "e2b",
  "browser",
  "headless",
];
const STATUSES: SubstrateStatus[] = ["idle", "running", "completed", "failed"];

export const Default: Story = {};

export const Matrix: Story = {
  render: () => (
    <div className="grid gap-3">
      {STATUSES.map((status) => (
        <div key={status} className="flex flex-wrap items-center gap-2">
          {SUBSTRATES.map((substrate) => (
            <SubstrateChip key={`${substrate}-${status}`} substrate={substrate} status={status} />
          ))}
        </div>
      ))}
    </div>
  ),
};

export const CompactMatrix: Story = {
  render: () => (
    <div className="grid gap-3">
      {STATUSES.map((status) => (
        <div key={status} className="flex flex-wrap items-center gap-2">
          {SUBSTRATES.map((substrate) => (
            <SubstrateChip
              key={`${substrate}-${status}`}
              substrate={substrate}
              status={status}
              compact
            />
          ))}
        </div>
      ))}
    </div>
  ),
};
