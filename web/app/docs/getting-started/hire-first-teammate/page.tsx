import type { Metadata } from "next";
import { DocsStub } from "../../_components/DocsStub";

export const metadata: Metadata = {
  title: "Hire your first AI teammate — SKYCOT Docs",
  description: "A guided walkthrough — workspace, provider, vertical, first hire.",
  alternates: { canonical: "/docs/getting-started/hire-first-teammate" },
};

export default function HireFirstTeammatePage() {
  return (
    <DocsStub
      slug="getting-started/hire-first-teammate"
      category="Getting Started"
      title="Hire your first AI teammate"
      intro={
        <>
          <p>
            Four steps stand between you and a working AI teammate — Workspace, Provider, Vertical,
            and Hire. The wizard remembers your answers so you can leave and return without losing
            progress.
          </p>
          <p>
            This guide walks the full flow with screenshots, plus what each choice locks in
            downstream (e.g. picking a vertical pre-loads the relevant skill library and approval
            policies for that role).
          </p>
        </>
      }
    />
  );
}
