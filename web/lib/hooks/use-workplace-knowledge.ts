"use client";

import { useMemo } from "react";
import { useSession } from "next-auth/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createKnowledgeDoc,
  deleteKnowledgeDoc,
  getKnowledgeDoc,
  listKnowledgeDocs,
  listKnowledgeRevisions,
  updateKnowledgeDoc,
  type CreateKnowledgeDocRequest,
  type KnowledgeCategory,
  type KnowledgeDoc,
  type ListKnowledgeResult,
  type ListRevisionsResult,
  type UpdateKnowledgeDocRequest,
} from "@/lib/api/workplace-knowledge";

/**
 * react-query wrappers for the workplace-knowledge endpoints. Matches the
 * shape of `use-api-keys.ts`: one `useQuery` per read, one `useMutation`
 * per write, optimistic cache updates for create / update / delete so the
 * UI feels instant.
 *
 * Query keys are arrays for the standard react-query partial-match shape:
 * - `["knowledge", "list"]` — list (no category filter)
 * - `["knowledge", "list", <category>]` — filtered list
 * - `["knowledge", "doc", <slug>]` — single doc head
 * - `["knowledge", "revisions", <slug>]` — version history
 *
 * RBAC selectors derive `canWrite` / `canDelete` from `session.role` —
 * the canonical workspace role string ("owner" | "admin" | "user" |
 * "guest"). Writes are Admin+, deletes Owner-only, matching the
 * gateway's `Action::KnowledgeWrite` / `Action::KnowledgeDelete` floors.
 */

export const KNOWLEDGE_QUERY_KEY = ["knowledge"] as const;

function listKey(category?: KnowledgeCategory) {
  return category ? (["knowledge", "list", category] as const) : (["knowledge", "list"] as const);
}

function docKey(slug: string) {
  return ["knowledge", "doc", slug] as const;
}

function revisionsKey(slug: string) {
  return ["knowledge", "revisions", slug] as const;
}

export function useKnowledgeDocs(category?: KnowledgeCategory) {
  return useQuery<ListKnowledgeResult>({
    queryKey: listKey(category),
    queryFn: () => listKnowledgeDocs(category),
  });
}

export function useKnowledgeDoc(slug: string | undefined) {
  return useQuery<KnowledgeDoc>({
    queryKey: docKey(slug ?? ""),
    queryFn: () => {
      if (!slug) throw new Error("slug required");
      return getKnowledgeDoc(slug);
    },
    enabled: Boolean(slug),
  });
}

export function useKnowledgeRevisions(slug: string | undefined) {
  return useQuery<ListRevisionsResult>({
    queryKey: revisionsKey(slug ?? ""),
    queryFn: () => {
      if (!slug) throw new Error("slug required");
      return listKnowledgeRevisions(slug);
    },
    enabled: Boolean(slug),
  });
}

export function useCreateKnowledgeDoc() {
  const qc = useQueryClient();
  return useMutation<KnowledgeDoc, Error, CreateKnowledgeDocRequest>({
    mutationFn: createKnowledgeDoc,
    onSuccess: (doc) => {
      qc.setQueryData<ListKnowledgeResult>(listKey(), (prev) => {
        if (!prev) return { docs: [doc], source: "gateway" };
        return { ...prev, docs: [doc, ...prev.docs] };
      });
      qc.setQueryData<ListKnowledgeResult>(listKey(doc.category), (prev) => {
        if (!prev) return { docs: [doc], source: "gateway" };
        return { ...prev, docs: [doc, ...prev.docs] };
      });
      qc.setQueryData<KnowledgeDoc>(docKey(doc.slug), doc);
    },
  });
}

export interface UpdateKnowledgeDocInput {
  slug: string;
  body_markdown: string;
  edit_reason?: string;
}

export function useUpdateKnowledgeDoc() {
  const qc = useQueryClient();
  return useMutation<KnowledgeDoc, Error, UpdateKnowledgeDocInput>({
    mutationFn: ({ slug, body_markdown, edit_reason }) => {
      const payload: UpdateKnowledgeDocRequest = { body_markdown };
      if (edit_reason) payload.edit_reason = edit_reason;
      return updateKnowledgeDoc(slug, payload);
    },
    onSuccess: (doc) => {
      qc.setQueryData<KnowledgeDoc>(docKey(doc.slug), doc);
      qc.invalidateQueries({ queryKey: revisionsKey(doc.slug) });
      const replaceInList = (prev: ListKnowledgeResult | undefined) => {
        if (!prev) return prev;
        return {
          ...prev,
          docs: prev.docs.map((d) => (d.slug === doc.slug ? doc : d)),
        };
      };
      qc.setQueryData<ListKnowledgeResult>(listKey(), replaceInList);
      qc.setQueryData<ListKnowledgeResult>(listKey(doc.category), replaceInList);
    },
  });
}

interface DeleteContext {
  prevAll: ListKnowledgeResult | undefined;
  prevByCategory: Map<KnowledgeCategory, ListKnowledgeResult | undefined>;
}

export function useDeleteKnowledgeDoc() {
  const qc = useQueryClient();
  return useMutation<{ ok: true; slug: string }, Error, string, DeleteContext>({
    mutationFn: deleteKnowledgeDoc,
    onMutate: async (slug): Promise<DeleteContext> => {
      await qc.cancelQueries({ queryKey: KNOWLEDGE_QUERY_KEY });
      const prevAll = qc.getQueryData<ListKnowledgeResult>(listKey());
      const prevByCategory = new Map<KnowledgeCategory, ListKnowledgeResult | undefined>();
      qc.setQueryData<ListKnowledgeResult>(listKey(), (curr) => {
        if (!curr) return curr;
        return { ...curr, docs: curr.docs.filter((d) => d.slug !== slug) };
      });
      for (const cat of [
        "brand",
        "business-rules",
        "compliance",
        "internal",
        "faq",
      ] as KnowledgeCategory[]) {
        const snap = qc.getQueryData<ListKnowledgeResult>(listKey(cat));
        prevByCategory.set(cat, snap);
        if (snap) {
          qc.setQueryData<ListKnowledgeResult>(listKey(cat), {
            ...snap,
            docs: snap.docs.filter((d) => d.slug !== slug),
          });
        }
      }
      return { prevAll, prevByCategory };
    },
    onError: (_err, _slug, ctx) => {
      if (!ctx) return;
      qc.setQueryData(listKey(), ctx.prevAll);
      for (const [cat, snap] of ctx.prevByCategory) {
        if (snap) qc.setQueryData(listKey(cat), snap);
      }
    },
  });
}

/** Per-action RBAC selectors. Owner can do anything; admin can write
 *  but not delete; user / guest are read-only. Default closed when the
 *  session role is missing (fail-safe matching the gateway). */
export interface KnowledgeViewerPolicy {
  role: string;
  canWrite: boolean;
  canDelete: boolean;
}

export function useKnowledgeViewerPolicy(): KnowledgeViewerPolicy {
  const { data: session } = useSession();
  const role = session?.role ?? "guest";
  return useMemo(
    () => ({
      role,
      canWrite: role === "owner" || role === "admin",
      canDelete: role === "owner",
    }),
    [role],
  );
}
