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
  const description = `${story.name} (${story.industry}, ${story.size}) — ${story.headlineKpi}`;
  const ogImage = `/customers/${slug}-og.png`;
  return {
    title,
    description,
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
    },
  };
}

export function generateStaticParams() {
  return SEED_CUSTOMERS.map((c) => ({ slug: c.slug }));
}

export default function CustomerStoryLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
