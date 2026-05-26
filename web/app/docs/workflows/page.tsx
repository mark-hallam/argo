import type { Metadata } from "next";
import { DocsStub } from "../_components/DocsStub";

export const metadata: Metadata = {
  title: "Workflows — SKYCOT Docs",
  description: "Build, edit, and version your AI workflows — visual canvas, plain-English editing, multi-branch logic, approval gates.",
  alternates: { canonical: "/docs/workflows" },
};

export default function WorkflowsPage() {
  return (
    <DocsStub
      slug="workflows"
      category="Workflows"
      title="Workflows"
      intro={
        <>
          <p>
            A workflow is a teammate&apos;s playbook for a specific job — &quot;handle inbound
            refund requests&quot;, &quot;follow up on stalled deals&quot;, &quot;triage support
            tickets&quot;. Workflows are versioned, branchable, and edited in either a visual canvas
            or plain English.
          </p>
          <p>
            This section covers the visual canvas, plain-English editing, multi-branch logic,
            approval-gate placement, and the workflow-vs-skill distinction.
          </p>
        </>
      }
    />
  );
}
