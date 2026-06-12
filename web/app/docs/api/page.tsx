import type { Metadata } from "next";
import { DocsStub } from "../_components/DocsStub";

export const metadata: Metadata = {
  title: "API Reference — SKYCOT Docs",
  description: "REST and WebSocket APIs for programmatic access to SKYCOT teammates, workflows, and audit data.",
  alternates: { canonical: "/docs/api" },
};

export default function ApiReferencePage() {
  return (
    <DocsStub
      slug="api"
      category="API Reference"
      title="API Reference"
      intro={
        <>
          <p>
            The SKYCOT API exposes everything you can do in the UI plus a few power-user
            primitives the UI doesn&apos;t surface. REST endpoints cover create/read/update/delete on
            teammates, workflows, skills, and audit records; WebSocket streams cover live
            audit-event subscriptions and approval-state changes.
          </p>
          <p>
            This section covers authentication (per-workspace API keys), the REST surface area,
            WebSocket event shapes, and rate-limit semantics.
          </p>
        </>
      }
    />
  );
}
