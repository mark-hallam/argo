import type { Metadata } from "next";
import type { ReactNode } from "react";

import { getCustomerStory, SEED_CUSTOMERS } from "@/lib/content/customers";

interface LayoutParams {
  params: Promise<{ slug: string }>;
}

export async function generateMetadata({ params }: LayoutParams): Promise<Metadata> {
  const { slug } = await params;
  const story = getCustomerStory(slug);
  if (!story) {
    return {
      title: "Customer not found",
      description: "The customer story you're looking for doesn't exist.",
    };
  }
  const title = `${story.name} — SKYCOT case study`;
  // Drops `size` from the descriptor — keeping name+industry+size+KPI ran
  // 157 chars for Polaris Marketing, over the 155-char SERP truncation
  // cap. Name + industry + KPI keeps every story ≤120 chars.
  const description = `${story.name} (${story.industry}) — ${story.headlineKpi}`;
  const ogImage = `/customers/${slug}-og.png`;
  return {
    title,
    description,
    // SEED_CUSTOMERS are illustrative until real case studies replace them;
    // block indexing of the per-customer detail pages until then. The
    // `/customers` index logo gallery stays indexable (see web/app/robots.ts).
    robots: { index: false, follow: false },
    alternates: {
      canonical: `/customers/${slug}`,
    },
    openGraph: {
      title: `${story.name} runs on SKYCOT`,
      description,
      images: [{ url: ogImage }],
    },
    twitter: {
      card: "summary_large_image",
      title: `${story.name} runs on SKYCOT`,
      description,
      images: [ogImage],
      site: "@skycot_ai",
      creator: "@skycot_ai",
    },
  };
}

export function generateStaticParams() {
  return SEED_CUSTOMERS.map((c) => ({ slug: c.slug }));
}

export default function CustomerStoryLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
