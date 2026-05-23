import type { KnowledgeCategory } from "@/lib/api/workplace-knowledge";

/** Display copy for each of the 5 workplace-knowledge categories. Shared
 *  by the CategoryNav, KnowledgeCard, and CreateDocModal so the labels
 *  stay consistent. */
export interface CategoryMeta {
  id: KnowledgeCategory;
  label: string;
  blurb: string;
}

export const CATEGORY_META: CategoryMeta[] = [
  { id: "brand", label: "Brand", blurb: "Voice, tone, do/don't, signature lines." },
  {
    id: "business-rules",
    label: "Business rules",
    blurb: "Refunds, escalation paths, SLA expectations.",
  },
  {
    id: "compliance",
    label: "Compliance",
    blurb: "Privacy, RBAC, regional obligations.",
  },
  {
    id: "internal",
    label: "Internal",
    blurb: "How we work — onboarding, glossary, runbooks.",
  },
  { id: "faq", label: "FAQ", blurb: "Top questions every worker is asked." },
];

export function categoryLabel(id: KnowledgeCategory): string {
  return CATEGORY_META.find((c) => c.id === id)?.label ?? id;
}
