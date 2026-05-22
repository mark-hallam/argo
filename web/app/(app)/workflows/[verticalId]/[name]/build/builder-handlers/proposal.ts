import type { Edge, Node } from "@xyflow/react";
import { useCallback, useState, type Dispatch, type SetStateAction } from "react";
import { toast } from "sonner";

import { stepsToFlow } from "../layout";
import type { WorkflowStep } from "../types";

export interface ProposalHandlerDeps {
  setNodes: Dispatch<SetStateAction<Node[]>>;
  setEdges: Dispatch<SetStateAction<Edge[]>>;
  clearSelectedNode: () => void;
}

export interface ProposalHandlers {
  promptText: string;
  setPromptText: Dispatch<SetStateAction<string>>;
  activeProposalPrompt: string | null;
  handleProposeChange: () => void;
  handlePickSuggestion: (next: string) => void;
  handleProposalApprove: (steps: WorkflowStep[]) => void;
  handleProposalCancel: () => void;
}

/**
 * AI-proposal panel state + handlers for the workflow builder.
 *
 * Owns the prompt textarea content and the currently-streaming proposal
 * prompt. Apply replaces the canvas via `setNodes` / `setEdges`, then
 * clears the prompt + selection so the user sees the post-apply graph.
 */
export function useProposalHandlers(deps: ProposalHandlerDeps): ProposalHandlers {
  const { setNodes, setEdges, clearSelectedNode } = deps;
  const [promptText, setPromptText] = useState("");
  const [activeProposalPrompt, setActiveProposalPrompt] = useState<string | null>(null);

  const handleProposeChange = useCallback(() => {
    const prompt = promptText.trim();
    if (!prompt) return;
    setActiveProposalPrompt(prompt);
  }, [promptText]);

  const handlePickSuggestion = useCallback((next: string) => {
    setPromptText(next);
    setActiveProposalPrompt(next.trim());
  }, []);

  const handleProposalApprove = useCallback(
    (steps: WorkflowStep[]) => {
      const next = stepsToFlow(steps);
      setNodes(next.nodes);
      setEdges(next.edges);
      clearSelectedNode();
      setActiveProposalPrompt(null);
      setPromptText("");
      toast.success("Proposal applied to canvas", {
        description: "Review the new graph, then Save + publish to persist.",
      });
    },
    [setNodes, setEdges, clearSelectedNode],
  );

  const handleProposalCancel = useCallback(() => {
    setActiveProposalPrompt(null);
  }, []);

  return {
    promptText,
    setPromptText,
    activeProposalPrompt,
    handleProposeChange,
    handlePickSuggestion,
    handleProposalApprove,
    handleProposalCancel,
  };
}
