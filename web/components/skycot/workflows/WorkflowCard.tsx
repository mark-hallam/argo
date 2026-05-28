"use client";

import { useRouter } from "next/navigation";
import { Bot, Layers, User } from "lucide-react";
import type { WorkflowAuthorType } from "@/lib/sample-workflows";

/**
 * Author chip rendered on the workflow card. Scene 5 of the v1 demo
 * narrates "workflows can be edited by the AI teammate themself" — the
 * card needs a visual anchor so the viewer can tell whether a row was
 * authored by an AI worker, a human teammate, or a template pack.
 *
 * Lives inside an outer `<Link>` on `/workflows`, so the worker
 * variant cannot itself be a `<Link>` (nested anchors are invalid
 * HTML). The button form intercepts the click, prevents the outer
 * navigation, and routes to `/workers/<id>` via `useRouter`.
 */
export interface WorkflowAuthorChipProps {
  authorType: WorkflowAuthorType;
  authorName: string;
  authorWorkerId?: string;
  className?: string;
}

const ICON_PROPS = { size: 12, "aria-hidden": true } as const;

export function WorkflowAuthorChip({
  authorType,
  authorName,
  authorWorkerId,
  className,
}: WorkflowAuthorChipProps) {
  const router = useRouter();
  const icon = renderIcon(authorType);
  const label = authorTypeLabel(authorType);
  const baseClass = "workflow-card-meta-item";
  const composed = className ? `${baseClass} ${className}` : baseClass;

  if (authorType === "worker" && authorWorkerId) {
    return (
      <button
        type="button"
        className={composed}
        data-testid="workflow-author-chip"
        data-author-type={authorType}
        aria-label={`${label} ${authorName} — open profile`}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          router.push(`/workers/${authorWorkerId}`);
        }}
        style={{
          background: "transparent",
          border: 0,
          padding: 0,
          font: "inherit",
          color: "inherit",
          letterSpacing: "inherit",
          textTransform: "inherit",
          cursor: "pointer",
        }}
      >
        {icon}
        by {authorName}
      </button>
    );
  }

  return (
    <span
      className={composed}
      data-testid="workflow-author-chip"
      data-author-type={authorType}
      aria-label={`${label} ${authorName}`}
    >
      {icon}
      by {authorName}
    </span>
  );
}

function renderIcon(authorType: WorkflowAuthorType) {
  switch (authorType) {
    case "worker":
      return <Bot {...ICON_PROPS} />;
    case "human":
      return <User {...ICON_PROPS} />;
    case "template_pack":
      return <Layers {...ICON_PROPS} />;
  }
}

function authorTypeLabel(authorType: WorkflowAuthorType): string {
  switch (authorType) {
    case "worker":
      return "AI teammate";
    case "human":
      return "Human teammate";
    case "template_pack":
      return "Template pack";
  }
}
