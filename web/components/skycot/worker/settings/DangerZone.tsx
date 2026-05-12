"use client";

import { useState } from "react";
import { Archive, PauseCircle, Trash2 } from "lucide-react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { TypeNameToConfirm } from "@/components/skycot/worker/settings/TypeNameToConfirm";
import type { WorkerPermissions } from "@/lib/types/worker";
import { cn } from "@/lib/utils";

/**
 * Danger zone — Pause / Archive / Delete. Pause + Archive are
 * destructive-outline; Delete is destructive-filled and gated by
 * `permissions.canDelete`. Delete uses TypeNameToConfirm; Archive
 * uses the standard AlertDialog with explicit consequences copy.
 */

export interface DangerZoneProps {
  workerName: string;
  permissions: WorkerPermissions;
  isPaused: boolean;
  onPause: () => void;
  onArchive: () => void;
  onDelete: () => void;
}

export function DangerZone({
  workerName,
  permissions,
  isPaused,
  onPause,
  onArchive,
  onDelete,
}: DangerZoneProps) {
  const [pausePending, setPausePending] = useState(false);
  const [archivePending, setArchivePending] = useState(false);
  const [deletePending, setDeletePending] = useState(false);

  return (
    <Card
      data-testid="card-settings-danger"
      className="border-[var(--destructive)]/30"
    >
      <CardHeader className={cn("border-t-2 border-[var(--destructive)]/40")}>
        <CardTitle className="text-[var(--destructive)]">Danger zone</CardTitle>
        <p className="text-sm text-[var(--muted-foreground)]">
          Actions here change {workerName}&apos;s availability or remove her entirely.
        </p>
      </CardHeader>
      <CardBody className="pt-1 space-y-2">
        <Button
          variant="destructive"
          size="sm"
          className="bg-transparent text-[var(--destructive)] border border-[var(--destructive)]/40 hover:bg-[var(--destructive)]/10"
          disabled={!permissions.canPauseResume}
          onClick={() => setPausePending(true)}
          data-testid="danger-pause"
        >
          <PauseCircle aria-hidden /> {isPaused ? "Resume" : "Pause"} {workerName}
        </Button>
        <Button
          variant="destructive"
          size="sm"
          className="bg-transparent text-[var(--destructive)] border border-[var(--destructive)]/40 hover:bg-[var(--destructive)]/10"
          disabled={!permissions.canArchive}
          onClick={() => setArchivePending(true)}
          data-testid="danger-archive"
        >
          <Archive aria-hidden /> Archive {workerName}
        </Button>
        <Button
          variant="destructive"
          size="sm"
          disabled={!permissions.canDelete}
          onClick={() => setDeletePending(true)}
          data-testid="danger-delete"
        >
          <Trash2 aria-hidden /> Delete {workerName}
        </Button>
        {!permissions.canDelete ? (
          <p
            data-testid="danger-delete-gated"
            className="text-xs text-[var(--muted-foreground)]"
          >
            Deleting a worker is an operator-only action.
          </p>
        ) : null}
      </CardBody>

      <AlertDialog
        open={pausePending}
        onOpenChange={setPausePending}
        title={isPaused ? `Resume ${workerName}?` : `Pause ${workerName}?`}
        description={
          isPaused
            ? `${workerName} will start picking up new work again.`
            : `${workerName} stops taking new actions until you resume. In-flight work is allowed to finish.`
        }
        confirmLabel={isPaused ? "Resume" : "Pause"}
        cancelLabel="Cancel"
        confirmVariant={isPaused ? "primary" : "destructive"}
        onConfirm={onPause}
      />

      <AlertDialog
        open={archivePending}
        onOpenChange={setArchivePending}
        title={`Archive ${workerName}?`}
        description={`Her memory and skills are preserved. You can recover her within 30 days.`}
        confirmLabel="Archive"
        cancelLabel="Cancel"
        confirmVariant="destructive"
        onConfirm={onArchive}
      />

      <TypeNameToConfirm
        open={deletePending}
        onOpenChange={setDeletePending}
        title={`Delete ${workerName} permanently?`}
        bodyLines={[
          "This cannot be undone.",
          `Audit logs remain but ${workerName}'s memory and learned skills are erased.`,
        ]}
        expectedName={workerName}
        cta="Delete"
        onConfirm={onDelete}
      />
    </Card>
  );
}
