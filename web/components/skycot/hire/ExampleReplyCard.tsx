"use client";

import { cn } from "@/lib/utils";
import type { WorkerExampleReply } from "@/lib/hire/draft-types";

/**
 * One inbound/outbound message pair, rendered as paired chat-bubble
 * cards under the worker draft preview.
 *
 * Layout:
 *  - Inbound (top-left, muted): the customer/operator message that
 *    arrives in the worker's inbox.
 *  - Outbound (bottom-right, primary): the worker's proposed reply in
 *    the user's voice.
 *
 * The example is purely illustrative — Step 4's hire commit doesn't
 * actually send these messages. The architectural target is server-
 * side moderation before display (`mission-enterprise-genesis-output-
 * moderation`); v1 trusts Genesis output as-is.
 */
export interface ExampleReplyCardProps {
  example: WorkerExampleReply;
  /** Worker's display name for the outbound bubble label. */
  workerName: string;
  index: number;
}

export function ExampleReplyCard({ example, workerName, index }: ExampleReplyCardProps) {
  return (
    <div
      data-testid={`hire-step4-example-${index}`}
      className="space-y-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-3"
    >
      <Bubble
        tone="inbound"
        speaker="Customer"
        body={example.inboundContext}
        testId={`hire-step4-example-${index}-inbound`}
      />
      <Bubble
        tone="outbound"
        speaker={workerName}
        body={example.mariahResponse}
        testId={`hire-step4-example-${index}-outbound`}
      />
    </div>
  );
}

interface BubbleProps {
  tone: "inbound" | "outbound";
  speaker: string;
  body: string;
  testId: string;
}

function Bubble({ tone, speaker, body, testId }: BubbleProps) {
  const inbound = tone === "inbound";
  return (
    <div className={cn("flex flex-col", inbound ? "items-start" : "items-end")}>
      <span className="mb-0.5 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">
        {speaker}
      </span>
      <p
        data-testid={testId}
        className={cn(
          "max-w-[80%] rounded-[var(--radius-md)] px-3 py-2 text-sm",
          inbound
            ? "bg-[var(--muted)] text-[var(--foreground)]"
            : "bg-[var(--primary)] text-[var(--primary-foreground)]",
        )}
      >
        {body}
      </p>
    </div>
  );
}
