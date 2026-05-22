import type { Edge, Node } from "@xyflow/react";
import { useCallback, useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";

import {
  useWorkflowSavePublish,
  type UseWorkflowSavePublishApi,
} from "@/lib/hooks/use-workflow-save-publish";

import { encodeStepsToToml } from "../step-encoder";
import type { WorkflowSpec, WorkflowStep } from "../types";

export interface SavePublishDeps {
  workflow: WorkflowSpec | undefined;
  verticalId: string;
  name: string;
  currentSteps: WorkflowStep[];
  nodes: Node[];
  edges: Edge[];
}

export interface SavePublishHandlers {
  savePublish: UseWorkflowSavePublishApi;
  savedDraftVersion: number | null;
  canPublish: boolean;
  publishConfirmOpen: boolean;
  setPublishConfirmOpen: (open: boolean) => void;
  handleSaveAndPublish: () => void;
  handlePublishRequest: () => void;
  handlePublishConfirm: () => void;
}

/**
 * Save-as-draft + publish-to-head handlers for the workflow builder.
 *
 * Owns the local "saved draft + canvas snapshot at save time" pair so
 * `canPublish` is true only when the canvas matches the most recently
 * saved draft. Diverged canvas → user must save again before publish.
 * This works around the phase-1 `useWorkflowBuilderState` not resetting
 * its own `dirty` baseline on save (out of scope for the builder
 * refactor — the local snapshot tracking is self-contained here).
 */
export function useSavePublishHandlers(deps: SavePublishDeps): SavePublishHandlers {
  const { workflow, verticalId, name, currentSteps, nodes, edges } = deps;
  const router = useRouter();
  const savePublish = useWorkflowSavePublish();

  const [savedDraft, setSavedDraft] = useState<
    { version_id: string; version: number; snapshot: string } | null
  >(null);
  const canvasSnapshot = useMemo(
    () => JSON.stringify({ nodes, edges }),
    [nodes, edges],
  );
  const canPublish =
    savedDraft !== null && savedDraft.snapshot === canvasSnapshot;
  const [publishConfirmOpen, setPublishConfirmOpen] = useState(false);

  const performSaveDraft = useCallback(async (): Promise<void> => {
    if (!workflow) return;
    const definitionToml = encodeStepsToToml(currentSteps, {
      workflowName: workflow.name,
      workflowDescription: workflow.description,
    });
    const snapshot = JSON.stringify({ nodes, edges });
    const result = await savePublish.saveDraft({
      vertical_id: verticalId,
      name,
      definition_toml: definitionToml,
    });
    setSavedDraft({
      version_id: result.version_id,
      version: result.version,
      snapshot,
    });
    toast.success(`Saved (v${result.version})`);
  }, [savePublish, currentSteps, workflow, verticalId, name, nodes, edges]);

  const handleSaveAndPublish = useCallback(() => {
    performSaveDraft().catch((err) => {
      const message = err instanceof Error && err.message
        ? err.message
        : "Network error";
      toast.error("Couldn't save workflow", {
        description: message,
        action: { label: "Retry", onClick: () => { void performSaveDraft(); } },
      });
    });
  }, [performSaveDraft]);

  const performPublishHead = useCallback(async (): Promise<void> => {
    if (!savedDraft) return;
    const result = await savePublish.publishHead({
      vertical_id: verticalId,
      name,
      version_id: savedDraft.version_id,
    });
    toast.success(`Published v${result.head_version} to head`);
    router.push(`/workflows/${verticalId}/${name}`);
  }, [savePublish, savedDraft, verticalId, name, router]);

  const handlePublishConfirm = useCallback(() => {
    setPublishConfirmOpen(false);
    performPublishHead().catch((err) => {
      const message = err instanceof Error && err.message
        ? err.message
        : "Network error";
      toast.error("Couldn't publish workflow", {
        description: message,
        action: {
          label: "Retry",
          onClick: () => { void performPublishHead(); },
        },
      });
    });
  }, [performPublishHead]);

  const handlePublishRequest = useCallback(() => {
    if (!canPublish) return;
    setPublishConfirmOpen(true);
  }, [canPublish]);

  return {
    savePublish,
    savedDraftVersion: savedDraft?.version ?? null,
    canPublish,
    publishConfirmOpen,
    setPublishConfirmOpen,
    handleSaveAndPublish,
    handlePublishRequest,
    handlePublishConfirm,
  };
}
