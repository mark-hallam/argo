import type { Metadata } from "next";
import { DocsStub } from "../../_components/DocsStub";

export const metadata: Metadata = {
  title: "Setting up Slack approval gates — SKYCOT Docs",
  description: "Route teammate-proposed actions to Slack for human approval before they execute.",
  alternates: { canonical: "/docs/integrations/slack-approvals" },
};

export default function SlackApprovalsPage() {
  return (
    <DocsStub
      slug="integrations/slack-approvals"
      category="Integrations"
      title="Setting up Slack approval gates"
      intro={
        <>
          <p>
            Slack approval gates let a SKYCOT teammate propose an action — a refund, a Salesforce
            update, a customer email — and pause until a designated human approves it from a Slack
            channel or DM (Direct Message).
          </p>
          <p>
            This guide covers installing the Slack app, picking the approver channel, mapping
            action types to gate policies, and the audit trail Slack approvals leave behind.
          </p>
        </>
      }
    />
  );
}
