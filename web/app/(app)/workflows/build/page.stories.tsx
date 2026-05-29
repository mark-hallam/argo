import type { Meta, StoryObj } from "@storybook/react";
import WorkflowBuildEntryPage from "./page";

/**
 * /workflows/build entry page — D3 brand-consistency recipe.
 *
 * Mirrors the static recipe pinned by `page.test.tsx`: Audiowide eyebrow
 * (`BUILD A WORKFLOW · CHOOSE A VERTICAL`), Audiowide hero with the
 * purple→pink gradient on "blank canvas", Inter weight-300 sub, and the
 * 5-vertical magazine cards in cohort-D3 register (first card is the
 * full-width lead tile).
 */
const meta: Meta<typeof WorkflowBuildEntryPage> = {
  title: "App/Workflows/Build entry",
  component: WorkflowBuildEntryPage,
  parameters: { layout: "fullscreen" },
};

export default meta;

type Story = StoryObj<typeof WorkflowBuildEntryPage>;

export const Default: Story = {
  name: "Brand-consistent entry (D3 recipe)",
};
