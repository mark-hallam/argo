/**
 * Workplace-knowledge API client (Stream `web-workplace-knowledge-editor`).
 *
 * Wraps the 6 RBAC-gated gateway endpoints landed in PR #2062 (Stream
 * `mission-workplace-knowledge-gateway`):
 *
 *   GET    /api/v1/knowledge[?category=<cat>]        (Guest+)
 *   GET    /api/v1/knowledge/<slug>                  (Guest+)
 *   POST   /api/v1/knowledge                         (Admin+)
 *   PUT    /api/v1/knowledge/<slug>                  (Admin+)
 *   DELETE /api/v1/knowledge/<slug>                  (Owner only)
 *   GET    /api/v1/knowledge/<slug>/revisions        (Guest+)
 *
 * The wire shape is the JSON emitted by `routes_knowledge_handlers.rs::doc_json`
 * and `revision_json`. Field names mirror the substrate (`updated_at_micros`,
 * `created_by`, …) — we surface them as-is rather than translating to ISO
 * strings client-side; renderers format epoch micros when displaying.
 *
 * Strict-production mode: when `NODE_ENV === "production"` AND
 * `NEXT_PUBLIC_ENV === "production"`, a 404 or unreachable gateway propagates
 * as an `ApiError`. Outside that envelope (dev / preview / Storybook), 404s
 * fall back to a small `SAMPLE_KNOWLEDGE` fixture so the page renders
 * meaningfully without a running gateway — same pattern as `lib/api/api-keys.ts`.
 */

import { apiDelete, apiGet, apiPost, apiPut, type ApiRequestInit } from "./client";
import { ApiError } from "./types";

export const KNOWLEDGE_CATEGORIES = [
  "brand",
  "business-rules",
  "compliance",
  "internal",
  "faq",
] as const;

export type KnowledgeCategory = (typeof KNOWLEDGE_CATEGORIES)[number];

export interface KnowledgeDoc {
  id: string;
  tenant_id: string;
  slug: string;
  title: string;
  category: KnowledgeCategory;
  body_markdown: string;
  version: number;
  parent_revision_id: string | null;
  created_at_micros: number;
  created_by: string;
  updated_at_micros: number;
  updated_by: string;
}

export interface KnowledgeRevision {
  id: string;
  doc_id: string;
  tenant_id: string;
  version: number;
  body_markdown: string;
  diff_from_prior: string | null;
  edited_at_micros: number;
  edited_by: string;
  edit_reason: string | null;
}

export interface CreateKnowledgeDocRequest {
  slug: string;
  title: string;
  category: KnowledgeCategory;
  body_markdown: string;
}

export interface UpdateKnowledgeDocRequest {
  body_markdown: string;
  edit_reason?: string;
}

export type KnowledgeDataSource = "gateway" | "mock-offline" | "mock-bypass";

export interface ListKnowledgeResult {
  docs: KnowledgeDoc[];
  source: KnowledgeDataSource;
}

export interface ListRevisionsResult {
  revisions: KnowledgeRevision[];
  source: KnowledgeDataSource;
}

export function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

/** Tiny fixture used in dev when the gateway is offline. Never returned
 *  in strict-production (acceptance #1 of design doc §7 Pillar 1). */
export const SAMPLE_KNOWLEDGE: KnowledgeDoc[] = [
  {
    id: "doc_sample_brand",
    tenant_id: "demo",
    slug: "brand-voice",
    title: "Brand voice",
    category: "brand",
    body_markdown:
      "# Brand voice\n\nWarm, clear, never breathless. " +
      "Avoid jargon. Lead with the customer's question, " +
      "not our product name.",
    version: 3,
    parent_revision_id: null,
    created_at_micros: Date.now() * 1000 - 7 * 24 * 60 * 60_000 * 1000,
    created_by: "sam@acmeco.com",
    updated_at_micros: Date.now() * 1000 - 2 * 60 * 60_000 * 1000,
    updated_by: "sam@acmeco.com",
  },
  {
    id: "doc_sample_refund",
    tenant_id: "demo",
    slug: "refund-policy",
    title: "Refund policy",
    category: "business-rules",
    body_markdown:
      "# Refund policy\n\nFull refund within 30 days of purchase. " +
      "Beyond 30 days requires admin approval.",
    version: 1,
    parent_revision_id: null,
    created_at_micros: Date.now() * 1000 - 14 * 24 * 60 * 60_000 * 1000,
    created_by: "olivia@acmeco.com",
    updated_at_micros: Date.now() * 1000 - 14 * 24 * 60 * 60_000 * 1000,
    updated_by: "olivia@acmeco.com",
  },
  {
    id: "doc_sample_pii",
    tenant_id: "demo",
    slug: "pii-handling",
    title: "PII handling",
    category: "compliance",
    body_markdown:
      "# PII handling\n\nNever log raw email or phone in audit payloads. " +
      "Use the redaction sink. Tax IDs are RBAC-gated.",
    version: 2,
    parent_revision_id: null,
    created_at_micros: Date.now() * 1000 - 3 * 24 * 60 * 60_000 * 1000,
    created_by: "olivia@acmeco.com",
    updated_at_micros: Date.now() * 1000 - 1 * 24 * 60 * 60_000 * 1000,
    updated_by: "olivia@acmeco.com",
  },
];

interface ListEnvelope {
  count: number;
  items: KnowledgeDoc[];
}

interface RevisionsEnvelope {
  slug: string;
  count: number;
  items: KnowledgeRevision[];
}

function buildListPath(category?: KnowledgeCategory): string {
  if (!category) return "/api/v1/knowledge";
  return `/api/v1/knowledge?category=${encodeURIComponent(category)}`;
}

export async function listKnowledgeDocs(
  category?: KnowledgeCategory,
  init?: ApiRequestInit,
): Promise<ListKnowledgeResult> {
  if (isAuthBypassed()) {
    return { docs: filterByCategory(SAMPLE_KNOWLEDGE, category), source: "mock-bypass" };
  }
  try {
    const env = await apiGet<ListEnvelope>(buildListPath(category), init);
    return { docs: env.items, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { docs: filterByCategory(SAMPLE_KNOWLEDGE, category), source: "mock-offline" };
    }
    throw err;
  }
}

export async function getKnowledgeDoc(
  slug: string,
  init?: ApiRequestInit,
): Promise<KnowledgeDoc> {
  if (isAuthBypassed()) {
    const sample = SAMPLE_KNOWLEDGE.find((d) => d.slug === slug);
    if (sample) return sample;
    throw new ApiError(404, "not found", "not_found");
  }
  try {
    return await apiGet<KnowledgeDoc>(
      `/api/v1/knowledge/${encodeURIComponent(slug)}`,
      init,
    );
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      const sample = SAMPLE_KNOWLEDGE.find((d) => d.slug === slug);
      if (sample) return sample;
    }
    throw err;
  }
}

export async function createKnowledgeDoc(
  input: CreateKnowledgeDocRequest,
): Promise<KnowledgeDoc> {
  if (isAuthBypassed()) {
    return mockDocFrom(input);
  }
  try {
    return await apiPost<KnowledgeDoc, CreateKnowledgeDocRequest>(
      "/api/v1/knowledge",
      input,
    );
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockDocFrom(input);
    }
    throw err;
  }
}

export async function updateKnowledgeDoc(
  slug: string,
  input: UpdateKnowledgeDocRequest,
): Promise<KnowledgeDoc> {
  if (isAuthBypassed()) {
    return mockUpdatedDoc(slug, input);
  }
  try {
    return await apiPut<KnowledgeDoc, UpdateKnowledgeDocRequest>(
      `/api/v1/knowledge/${encodeURIComponent(slug)}`,
      input,
    );
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockUpdatedDoc(slug, input);
    }
    throw err;
  }
}

export async function deleteKnowledgeDoc(slug: string): Promise<{ ok: true; slug: string }> {
  if (isAuthBypassed()) return { ok: true, slug };
  try {
    await apiDelete<unknown>(`/api/v1/knowledge/${encodeURIComponent(slug)}`);
    return { ok: true, slug };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { ok: true, slug };
    }
    throw err;
  }
}

export async function listKnowledgeRevisions(
  slug: string,
  init?: ApiRequestInit,
): Promise<ListRevisionsResult> {
  if (isAuthBypassed()) {
    return { revisions: mockRevisions(slug), source: "mock-bypass" };
  }
  try {
    const env = await apiGet<RevisionsEnvelope>(
      `/api/v1/knowledge/${encodeURIComponent(slug)}/revisions`,
      init,
    );
    return { revisions: env.items, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { revisions: mockRevisions(slug), source: "mock-offline" };
    }
    throw err;
  }
}

function filterByCategory(
  docs: KnowledgeDoc[],
  category?: KnowledgeCategory,
): KnowledgeDoc[] {
  if (!category) return docs;
  return docs.filter((d) => d.category === category);
}

function mockDocFrom(input: CreateKnowledgeDocRequest): KnowledgeDoc {
  const now = Date.now() * 1000;
  return {
    id: `doc_${Date.now().toString(36)}`,
    tenant_id: "demo",
    slug: input.slug,
    title: input.title,
    category: input.category,
    body_markdown: input.body_markdown,
    version: 1,
    parent_revision_id: null,
    created_at_micros: now,
    created_by: "mock-user",
    updated_at_micros: now,
    updated_by: "mock-user",
  };
}

function mockUpdatedDoc(slug: string, input: UpdateKnowledgeDocRequest): KnowledgeDoc {
  const now = Date.now() * 1000;
  const seed = SAMPLE_KNOWLEDGE.find((d) => d.slug === slug);
  return {
    id: seed?.id ?? `doc_${Date.now().toString(36)}`,
    tenant_id: "demo",
    slug,
    title: seed?.title ?? slug,
    category: seed?.category ?? "internal",
    body_markdown: input.body_markdown,
    version: (seed?.version ?? 0) + 1,
    parent_revision_id: null,
    created_at_micros: seed?.created_at_micros ?? now,
    created_by: seed?.created_by ?? "mock-user",
    updated_at_micros: now,
    updated_by: "mock-user",
  };
}

function mockRevisions(slug: string): KnowledgeRevision[] {
  const seed = SAMPLE_KNOWLEDGE.find((d) => d.slug === slug);
  if (!seed) return [];
  return Array.from({ length: seed.version }, (_, i) => ({
    id: `rev_${slug}_${i + 1}`,
    doc_id: seed.id,
    tenant_id: seed.tenant_id,
    version: seed.version - i,
    body_markdown: seed.body_markdown,
    diff_from_prior: i === seed.version - 1 ? null : "- prior\n+ updated",
    edited_at_micros: seed.updated_at_micros - i * 24 * 60 * 60_000 * 1000,
    edited_by: seed.updated_by,
    edit_reason: i === 0 ? "Latest edit" : null,
  }));
}
