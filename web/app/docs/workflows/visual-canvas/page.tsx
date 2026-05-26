import type { Metadata } from "next";
import { DocsStub } from "../../_components/DocsStub";

export const metadata: Metadata = {
  title: "The workflow builder, end to end — SKYCOT Docs",
  description: "A nine-minute walkthrough of the visual canvas — nodes, edges, branches, approvals, and the plain-English overlay.",
  alternates: { canonical: "/docs/workflows/visual-canvas" },
};

export default function VisualCanvasPage() {
  return (
    <DocsStub
      slug="workflows/visual-canvas"
      category="Workflows"
      title="The workflow builder, end to end"
      intro={
        <>
          <p>
            The visual canvas is where you assemble a teammate&apos;s playbook. Each node is an
            action — a connector call, a skill invocation, a branch on data, or an approval gate.
            Edges are the order in which they fire.
          </p>
          <p>
            This guide takes a real example — a Tier 1 support triage workflow — and builds it
            end-to-end so you can see how the canvas, the plain-English overlay, and the live
            preview interact.
          </p>
        </>
      }
    />
  );
}
