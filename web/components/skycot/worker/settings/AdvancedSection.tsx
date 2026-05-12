"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useCallback, useState } from "react";
import Link from "next/link";
import { ClipboardCopy, FileCode, History, PlaySquare } from "lucide-react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button, buttonVariants } from "@/components/ui/Button";
import type { AdvancedConfig } from "@/lib/api/worker-settings";
import { cn } from "@/lib/utils";

/**
 * Advanced section — operator-only. Gated at the page level by
 * `permissions.canViewSettings`; if a delegator somehow lands here
 * the section is hidden upstream.
 *
 * v1 surfaces:
 *  - Read-only TOML manifest viewer (modal).
 *  - Trajectory-replay sandbox placeholder (links to E5).
 *  - Worker ID + provenance chip with copy-on-click for each id.
 *  - Audit-the-audit log deep-link into the audit timeline filtered
 *    to settings changes for this worker.
 */

export interface AdvancedSectionProps {
  workerId: string;
  advanced: AdvancedConfig;
}

export function AdvancedSection({ workerId, advanced }: AdvancedSectionProps) {
  const [manifestOpen, setManifestOpen] = useState(false);
  const [copied, setCopied] = useState<string | null>(null);

  const onCopy = useCallback((label: string, value: string) => {
    setCopied(label);
    try {
      navigator.clipboard?.writeText(value);
    } catch {
      // ignore in environments without the API (e.g. jsdom)
    }
    setTimeout(() => setCopied(null), 1200);
  }, []);

  const auditHref = `/audit?worker=${encodeURIComponent(workerId)}&action_type=settings_change`;
  const replayHref = `/audit?worker=${encodeURIComponent(workerId)}&action_type=trajectory_replay`;

  return (
    <Card data-testid="card-settings-advanced">
      <CardHeader>
        <CardTitle>Advanced</CardTitle>
        <p className="text-sm text-[var(--muted-foreground)]">
          Operator-only diagnostics, manifest viewer, and audit access.
        </p>
      </CardHeader>
      <CardBody className="pt-1 space-y-3">
        <div className="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            size="sm"
            data-testid="advanced-view-manifest"
            onClick={() => setManifestOpen(true)}
          >
            <FileCode aria-hidden /> View role manifest (TOML)
          </Button>
          <Link
            href={replayHref}
            data-testid="advanced-trajectory-replay"
            className={buttonVariants({ variant: "secondary", size: "sm" })}
          >
            <PlaySquare aria-hidden /> Trajectory replay sandbox
          </Link>
          <Link
            href={auditHref}
            data-testid="advanced-audit-link"
            className={buttonVariants({ variant: "secondary", size: "sm" })}
          >
            <History aria-hidden /> Audit-the-audit log
          </Link>
        </div>

        <ProvenanceChip
          label="Worker ID"
          value={workerId}
          testid="provenance-worker"
          onCopy={onCopy}
        />
        <ProvenanceChip
          label="Tenant ID"
          value={advanced.tenantId}
          testid="provenance-tenant"
          onCopy={onCopy}
        />
        <ProvenanceChip
          label="Namespace ID"
          value={advanced.namespaceId}
          testid="provenance-namespace"
          onCopy={onCopy}
        />
        <ProvenanceChip
          label="Version hash"
          value={advanced.versionHash}
          testid="provenance-version"
          onCopy={onCopy}
        />

        <div
          role="status"
          aria-live="polite"
          data-testid="advanced-copy-announcement"
          className="sr-only"
        >
          {copied ? `${copied} copied` : ""}
        </div>
      </CardBody>

      <DialogPrimitive.Root open={manifestOpen} onOpenChange={setManifestOpen}>
        <DialogPrimitive.Portal>
          <DialogPrimitive.Overlay
            className={cn(
              "fixed inset-0 z-40 bg-black/40 backdrop-blur-sm",
              "data-[state=open]:animate-in data-[state=closed]:animate-out",
            )}
          />
          <DialogPrimitive.Content
            data-testid="manifest-dialog"
            className={cn(
              "fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-2xl -translate-x-1/2 -translate-y-1/2",
              "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg",
              "focus:outline-none",
            )}
          >
            <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
              Role manifest (read-only)
            </DialogPrimitive.Title>
            <DialogPrimitive.Description className="mt-1 text-sm text-[var(--muted-foreground)]">
              Editing creates a new version + audit entry. Ships with
              <code className="ml-1 font-mono text-xs">mission-enterprise-role-manifest-versioning</code>.
            </DialogPrimitive.Description>
            <pre
              data-testid="manifest-toml"
              className="mt-3 max-h-[60vh] overflow-auto rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/30 p-3 font-mono text-xs text-[var(--foreground)]"
            >
              {advanced.manifestToml}
            </pre>
            <div className="mt-4 flex justify-end">
              <Button
                variant="ghost"
                size="sm"
                data-testid="manifest-close"
                onClick={() => setManifestOpen(false)}
              >
                Close
              </Button>
            </div>
          </DialogPrimitive.Content>
        </DialogPrimitive.Portal>
      </DialogPrimitive.Root>
    </Card>
  );
}

interface ProvenanceChipProps {
  label: string;
  value: string;
  testid: string;
  onCopy: (label: string, value: string) => void;
}

function ProvenanceChip({ label, value, testid, onCopy }: ProvenanceChipProps) {
  return (
    <div
      data-testid={testid}
      className="flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/30 px-3 py-1.5 text-xs"
    >
      <span className="font-medium text-[var(--muted-foreground)]">{label}</span>
      <code className="flex-1 truncate font-mono text-[var(--foreground)]">{value}</code>
      <button
        type="button"
        onClick={() => onCopy(label, value)}
        aria-label={`Copy ${label}`}
        data-testid={`${testid}-copy`}
        className="rounded-[var(--radius-sm)] p-1 hover:bg-[var(--accent)]/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
      >
        <ClipboardCopy className="size-3.5" aria-hidden />
      </button>
    </div>
  );
}
