import type { Meta, StoryObj } from "@storybook/react";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";

/**
 * Real prop fixtures lifted from the four production callsites:
 * - /team (web/app/(app)/team/page.tsx)
 * - /cohorts (web/app/(app)/cohorts/page.tsx)
 * - /approvals (web/app/(app)/approvals/page.tsx)
 * - /hire/done/[workerId] (web/components/skycot/hire/HireDoneCelebration.tsx)
 *
 * Stories use those exact prop values so the visual contract here
 * matches what ships in the app — no decorative placeholder copy.
 */

const meta: Meta<typeof BrandLoudHero> = {
  title: "Skycot/BrandLoudHero",
  component: BrandLoudHero,
  parameters: { layout: "fullscreen" },
};

export default meta;

type Story = StoryObj<typeof BrandLoudHero>;

export const TeamLive: Story = {
  name: "Team — workforce live (pulse dot + KPIs)",
  args: {
    eyebrowDotColor: "#f0fdf4",
    eyebrowLabel: "YOUR AI WORKFORCE · LIVE",
    pulseDot: true,
    title: "4 teammates, one team.",
    sub: "3 working right now · 1,284 actions this week · saving you ~38 hours.",
    subMaxWidth: 420,
    shineDurationS: 4.5,
    kpis: [
      { label: "Working", value: "3" },
      { label: "Actions/wk", value: "1,284" },
      { label: "Hours saved", value: "38" },
    ],
  },
};

export const TeamEmpty: Story = {
  name: "Team — empty state (no KPIs)",
  args: {
    eyebrowDotColor: "#f0fdf4",
    eyebrowLabel: "YOUR AI WORKFORCE · LIVE",
    pulseDot: true,
    title: "Hire your first teammate.",
    sub: "No teammates yet. Pick a vertical and they'll be in your Slack in four minutes.",
    subMaxWidth: 420,
    shineDurationS: 4.5,
  },
};

export const Cohorts: Story = {
  name: "Cohorts — multi cohort (KPIs, no pulse)",
  args: {
    eyebrowDotColor: "#ddd6fe",
    eyebrowLabel: "COHORTS",
    title: "7 cohorts across your fleet.",
    sub: "Each cohort earns its trust grade by shipping consistently. Promotions happen automatically.",
    subMaxWidth: 480,
    shineDurationS: 6,
    kpis: [
      { label: "Cohorts", value: "7" },
      { label: "Members", value: "32" },
      { label: "Skills", value: "118" },
    ],
  },
};

export const ApprovalsInbox: Story = {
  name: "Approvals — inbox (no KPIs, no pulse)",
  args: {
    eyebrowDotColor: "#fbbf24",
    eyebrowLabel: "INBOX",
    title: "Approvals.",
    sub: "Every decision your AI teammates queue for a human shows up here — refunds, hires, contracts, anything that needs a sign-off.",
  },
};

export const HireDoneWelcome: Story = {
  name: "Hire/done — welcome (titleAccent + pulse)",
  args: {
    eyebrowDotColor: "#fef3c7",
    eyebrowLabel: "JUST HIRED · WELCOME",
    pulseDot: true,
    title: "Meet Mariah.",
    titleAccent: "Mariah",
    sub: "Your newest teammate is on the bench and ready to start. Say hi in Slack or jump into their profile to dial in their first week.",
    subMaxWidth: 520,
    shineDurationS: 4,
  },
};
