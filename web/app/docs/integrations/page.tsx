import type { Metadata } from "next";
import { DocsStub } from "../_components/DocsStub";

export const metadata: Metadata = {
  title: "Integrations — SKYCOT Docs",
  description: "Slack, Salesforce, Workday, Linear, GitHub, Stripe, and 20+ more — connector configuration and approval-aware wiring.",
  alternates: { canonical: "/docs/integrations" },
};

export default function IntegrationsPage() {
  return (
    <DocsStub
      slug="integrations"
      category="Integrations"
      title="Integrations"
      intro={
        <>
          <p>
            SKYCOT teammates work through the same tools your humans already use. Connect Slack for
            approval pings, Salesforce or HubSpot for CRM (Customer Relationship Management — the
            system that tracks customers and deals) writes, Linear or GitHub for engineering work,
            and 20+ more connectors via the marketplace.
          </p>
          <p>
            Every connector ships with approval-aware wiring out of the box — destructive actions
            route through your configured approval gate before they hit the third-party system.
          </p>
        </>
      }
    />
  );
}
