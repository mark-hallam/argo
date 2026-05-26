import type { Metadata } from "next";
import { DocsStub } from "../_components/DocsStub";

export const metadata: Metadata = {
  title: "Skills — SKYCOT Docs",
  description: "The skills marketplace, authoring custom skills, and tenant-scoped skill libraries.",
  alternates: { canonical: "/docs/skills" },
};

export default function SkillsPage() {
  return (
    <DocsStub
      slug="skills"
      category="Skills"
      title="Skills"
      intro={
        <>
          <p>
            Skills are the reusable capabilities a teammate carries — drafting a refund decision,
            writing a status report, summarizing a long Slack thread. Browse the public skills
            marketplace, install one with a click, or author your own and share it across your
            workspace.
          </p>
          <p>
            This section covers installing public skills, authoring custom skills as
            plain-English bundles, tenant scoping, and how teammates learn from successful skill
            invocations.
          </p>
        </>
      }
    />
  );
}
