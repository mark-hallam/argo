import type { Metadata } from "next";
import { DocsStub } from "../_components/DocsStub";

export const metadata: Metadata = {
  title: "Audit & Compliance — SKYCOT Docs",
  description: "Forensic-grade audit chain on every action — RBAC, workspace isolation, SOC 2-ready evidence, kill-switch ops.",
  alternates: { canonical: "/docs/audit" },
};

export default function AuditPage() {
  return (
    <DocsStub
      slug="audit"
      category="Audit & Compliance"
      title="Audit & Compliance"
      intro={
        <>
          <p>
            Every action a SKYCOT teammate takes — every connector call, every approval decision,
            every memory write — lands in a forensic-grade audit chain. The chain is tenant-scoped,
            tamper-evident, and structured so SOC 2 (Service Organization Control 2 — an enterprise
            security audit standard) evidence requests can be served from it directly.
          </p>
          <p>
            This section covers per-action RBAC (Role-Based Access Control — who can do what),
            workspace isolation, the audit event schema, evidence-export flows, and the kill-switch
            for operator emergencies.
          </p>
        </>
      }
    />
  );
}
