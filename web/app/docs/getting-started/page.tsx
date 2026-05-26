import type { Metadata } from "next";
import { DocsStub } from "../_components/DocsStub";

export const metadata: Metadata = {
  title: "Getting Started — SKYCOT Docs",
  description: "Hire your first AI teammate in four minutes — onboarding, trust ratchet, and your first workflow.",
  alternates: { canonical: "/docs/getting-started" },
};

export default function GettingStartedPage() {
  return (
    <DocsStub
      slug="getting-started"
      category="Getting Started"
      title="Getting Started"
      intro={
        <>
          <p>
            Welcome to SKYCOT. The fastest way to understand the platform is to hire one AI
            (Artificial Intelligence — software that performs tasks usually requiring human judgement)
            teammate, watch it complete a real piece of work, then promote it through the trust ratchet
            as you grow confident in its output.
          </p>
          <p>
            This section covers onboarding your workspace, picking your first vertical, hiring your
            first teammate, and the trust-ratchet escalation model.
          </p>
        </>
      }
    />
  );
}
