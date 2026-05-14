"use client";

import { useCallback } from "react";
import { useParams, useRouter } from "next/navigation";
import { Breadcrumb, BreadcrumbItem, BreadcrumbSeparator } from "@/components/ui/Breadcrumb";
import { GhostwriterForm } from "@/components/skycot/canvas/GhostwriterForm";
import type { GhostwriterDraft } from "@/lib/api/ghostwriter";

/**
 * Ghostwriter entry surface — `/workers/<workerId>/skills/new` (L6).
 *
 * Renders the prompt form; on success redirects to the L4 read-only
 * canvas at `/workers/<workerId>/skills/<skillId>/canvas?version=<v>`
 * so the operator immediately sees the drafted IR instead of a blank
 * grid. The L4 viewer page already handles the "no IR yet" empty
 * state, so a follow-up edit landing on L7 will reuse the same route.
 *
 * Sensible deviation from L6 brief: brief listed
 * `web/app/(app)/workers/[id]/skills/new/page.tsx`, but the live
 * route segment is `[workerId]` (matches the rest of the worker tree
 * — see L4 page for the same correction in its sensible-deviations
 * note). Using `[id]` would have created a parallel param name.
 */
export default function NewSkillPage() {
  const params = useParams<{ workerId: string }>();
  const workerId = params?.workerId ?? "";
  const router = useRouter();

  const onDraftReady = useCallback(
    (draft: GhostwriterDraft) => {
      const path = `/workers/${workerId}/skills/${draft.skillId}/canvas?version=${encodeURIComponent(
        draft.version,
      )}`;
      router.push(path);
    },
    [router, workerId],
  );

  return (
    <div data-testid="ghostwriter-page" className="space-y-4">
      <Breadcrumb>
        <BreadcrumbItem href={`/workers/${workerId}/skills`}>Skills</BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem current>New</BreadcrumbItem>
      </Breadcrumb>

      <header className="space-y-1">
        <h1 className="text-xl font-semibold text-[var(--foreground)]">
          Generate a skill from a prompt
        </h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          Describe the work in plain English. We&apos;ll draft a workflow
          and open it on the canvas for you to refine.
        </p>
      </header>

      <GhostwriterForm onDraftReady={onDraftReady} />
    </div>
  );
}
