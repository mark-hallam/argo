import type { JSX } from "react";

/**
 * Renders Schema.org JSON-LD as inert <script type="application/ld+json"> tags.
 * Search engines (Google, Bing, DuckDuckGo) parse these blocks for rich-snippet
 * eligibility — sitelinks, FAQ accordions, product pricing, breadcrumbs.
 *
 * The component is a server component (no "use client") so it can be embedded
 * in any layout.tsx without forcing the surrounding tree client-side. JSON is
 * stringified once at render and HTML-escaped at the literal "<" character to
 * prevent script-context break-out.
 *
 * Add new schema shapes here as needed (TechArticle, BreadcrumbList, etc.).
 */

type SchemaContext = "https://schema.org";

interface SchemaBase {
  "@context": SchemaContext;
  "@type": string;
  [key: string]: unknown;
}

export interface OrganizationSchema extends SchemaBase {
  "@type": "Organization";
  name: string;
  url: string;
  logo?: string;
  description?: string;
  sameAs?: string[];
  contactPoint?: {
    "@type": "ContactPoint";
    email?: string;
    contactType?: string;
    url?: string;
  };
  foundingDate?: string;
  foundingLocation?: { "@type": "Place"; name: string };
}

export interface OfferSchema {
  "@type": "Offer";
  name?: string;
  price: string;
  priceCurrency: string;
  url?: string;
  availability?: string;
  category?: string;
}

export interface AggregateOfferSchema {
  "@type": "AggregateOffer";
  priceCurrency: string;
  lowPrice: string;
  highPrice?: string;
  offerCount?: number;
  offers?: OfferSchema[];
}

export interface ProductSchema extends SchemaBase {
  "@type": "Product" | "SoftwareApplication";
  name: string;
  description: string;
  brand: { "@type": "Brand"; name: string };
  url?: string;
  image?: string;
  offers?: OfferSchema | OfferSchema[] | AggregateOfferSchema;
  applicationCategory?: string;
  operatingSystem?: string;
}

export interface FaqQuestionSchema {
  "@type": "Question";
  name: string;
  acceptedAnswer: { "@type": "Answer"; text: string };
}

export interface FaqPageSchema extends SchemaBase {
  "@type": "FAQPage";
  mainEntity: FaqQuestionSchema[];
}

export interface ArticleSchema extends SchemaBase {
  "@type": "Article" | "NewsArticle" | "BlogPosting" | "TechArticle";
  headline: string;
  datePublished: string;
  dateModified?: string;
  description?: string;
  url?: string;
  author?: { "@type": "Organization" | "Person"; name: string; url?: string };
  publisher?: {
    "@type": "Organization";
    name: string;
    logo?: { "@type": "ImageObject"; url: string };
  };
  keywords?: string[];
  articleSection?: string;
}

export type Schema =
  | OrganizationSchema
  | ProductSchema
  | FaqPageSchema
  | ArticleSchema
  | SchemaBase;

interface JsonLdProps {
  data: Schema | Schema[];
}

export function serializeJsonLd(item: Schema): string {
  return JSON.stringify(item).replace(/</g, "\\u003c");
}

export function JsonLd({ data }: JsonLdProps): JSX.Element {
  const items = Array.isArray(data) ? data : [data];
  return (
    <>
      {items.map((item, i) => (
        <script
          // Static ordering, no reordering across renders.
          // eslint-disable-next-line react/no-array-index-key
          key={i}
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: serializeJsonLd(item) }}
        />
      ))}
    </>
  );
}

const SITE_URL = "https://skycot.com";
const ORG_NAME = "SKYCOT";
const ORG_LOGO = `${SITE_URL}/icon.svg`;
const ORG_DESCRIPTION =
  "AI teammates that live in Slack, learn from your feedback, and ask for approval before they act. Audit-first, single-binary, enterprise-grade.";

export const ORGANIZATION_SCHEMA: OrganizationSchema = {
  "@context": "https://schema.org",
  "@type": "Organization",
  name: ORG_NAME,
  url: SITE_URL,
  logo: ORG_LOGO,
  description: ORG_DESCRIPTION,
  foundingDate: "2026",
  foundingLocation: { "@type": "Place", name: "Melbourne, Australia" },
  sameAs: ["https://github.com/mark-hallam/skycot-packs"],
  contactPoint: {
    "@type": "ContactPoint",
    email: "hello@skycot.com",
    contactType: "customer support",
    url: `${SITE_URL}/help`,
  },
};

export function organizationSchema(): OrganizationSchema {
  return ORGANIZATION_SCHEMA;
}

interface ProductOptions {
  url: string;
  priceCurrency?: string;
  lowPrice: string;
  highPrice?: string;
  offers?: OfferSchema[];
}

export function softwareApplicationSchema(opts: ProductOptions): ProductSchema {
  return {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: ORG_NAME,
    description: ORG_DESCRIPTION,
    brand: { "@type": "Brand", name: ORG_NAME },
    url: opts.url,
    image: ORG_LOGO,
    applicationCategory: "BusinessApplication",
    operatingSystem: "Web, macOS, Linux",
    offers: {
      "@type": "AggregateOffer",
      priceCurrency: opts.priceCurrency ?? "USD",
      lowPrice: opts.lowPrice,
      highPrice: opts.highPrice,
      offerCount: opts.offers?.length,
      offers: opts.offers,
    },
  };
}

export interface FaqEntry {
  question: string;
  answer: string;
}

export function faqPageSchema(entries: FaqEntry[]): FaqPageSchema {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: entries.map((e) => ({
      "@type": "Question",
      name: e.question,
      acceptedAnswer: { "@type": "Answer", text: e.answer },
    })),
  };
}

export interface ArticleEntry {
  headline: string;
  datePublished: string;
  description?: string;
  url?: string;
  keywords?: string[];
}

export function articleSchema(entry: ArticleEntry): ArticleSchema {
  return {
    "@context": "https://schema.org",
    "@type": "Article",
    headline: entry.headline,
    datePublished: entry.datePublished,
    dateModified: entry.datePublished,
    description: entry.description,
    url: entry.url,
    keywords: entry.keywords,
    articleSection: "Changelog",
    author: { "@type": "Organization", name: ORG_NAME, url: SITE_URL },
    publisher: {
      "@type": "Organization",
      name: ORG_NAME,
      logo: { "@type": "ImageObject", url: ORG_LOGO },
    },
  };
}
