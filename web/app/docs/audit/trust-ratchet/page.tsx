import type { Metadata } from "next";
import { DocsStub } from "../../_components/DocsStub";

export const metadata: Metadata = {
  title: "How the trust ratchet promotes workers — SKYCOT Docs",
  description: "The promotion model — propose-with-approval to autonomous-within-policy as confidence accumulates.",
  alternates: { canonical: "/docs/audit/trust-ratchet" },
};

export default function TrustRatchetPage() {
  return (
    <DocsStub
      slug="audit/trust-ratchet"
      category="Audit & Compliance"
      title="How the trust ratchet promotes workers"
      intro={
        <>
          <p>
            A SKYCOT teammate starts at the lowest trust level — every action it proposes routes
            through a human approval gate. As approvals accumulate and audited outcomes look clean,
            the teammate is promoted up the ratchet: from propose-with-approval, to
            propose-with-sampled-approval, to autonomous-within-policy.
          </p>
          <p>
            This guide explains the promotion criteria, the policies that bound each level, how to
            review a teammate&apos;s ratchet history, and how to demote a teammate when the audit
            signals say you should.
          </p>
        </>
      }
    />
  );
}
