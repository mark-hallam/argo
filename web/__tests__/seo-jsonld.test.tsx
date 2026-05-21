import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  ORGANIZATION_SCHEMA,
  articleSchema,
  faqPageSchema,
  JsonLd,
  organizationSchema,
  serializeJsonLd,
  softwareApplicationSchema,
} from "@/components/seo/JsonLd";

function extractJsonLd(container: HTMLElement): Record<string, unknown>[] {
  const scripts = container.querySelectorAll<HTMLScriptElement>(
    'script[type="application/ld+json"]',
  );
  return Array.from(scripts).map((s) =>
    JSON.parse(s.textContent ?? "") as Record<string, unknown>,
  );
}

describe("JsonLd component", () => {
  it("renders a single <script type=application/ld+json> with valid JSON", () => {
    const data = organizationSchema();
    const { container } = render(<JsonLd data={data} />);
    const parsed = extractJsonLd(container);
    expect(parsed).toHaveLength(1);
    expect(parsed[0]["@context"]).toBe("https://schema.org");
    expect(parsed[0]["@type"]).toBe("Organization");
  });

  it("renders one script per item when passed an array", () => {
    const data = [organizationSchema(), faqPageSchema([])];
    const { container } = render(<JsonLd data={data} />);
    const parsed = extractJsonLd(container);
    expect(parsed).toHaveLength(2);
    expect(parsed[0]["@type"]).toBe("Organization");
    expect(parsed[1]["@type"]).toBe("FAQPage");
  });

  it("escapes literal < to prevent breaking out of the script context", () => {
    const malicious = {
      "@context": "https://schema.org" as const,
      "@type": "Article",
      headline: "<script>alert('xss')</script>",
    };
    const serialized = serializeJsonLd(malicious);
    expect(serialized).not.toMatch(/<script/);
    expect(serialized).toMatch(/\\u003c/);
    // Still parses as valid JSON.
    expect(JSON.parse(serialized).headline).toBe("<script>alert('xss')</script>");
  });
});

describe("organizationSchema()", () => {
  it("ships the canonical SKYCOT identity", () => {
    expect(ORGANIZATION_SCHEMA.name).toBe("SKYCOT");
    expect(ORGANIZATION_SCHEMA.url).toBe("https://skycot.com");
    expect(ORGANIZATION_SCHEMA.logo).toMatch(/skycot\.com\//);
    expect(ORGANIZATION_SCHEMA.contactPoint?.email).toBe("hello@skycot.com");
  });

  it("returns the same canonical schema object", () => {
    expect(organizationSchema()).toBe(ORGANIZATION_SCHEMA);
  });
});

describe("softwareApplicationSchema()", () => {
  it("includes brand, offers, and category", () => {
    const schema = softwareApplicationSchema({
      url: "https://skycot.com/landing",
      lowPrice: "0",
      highPrice: "29",
      offers: [
        {
          "@type": "Offer",
          name: "Free",
          price: "0",
          priceCurrency: "USD",
        },
      ],
    });
    expect(schema["@type"]).toBe("SoftwareApplication");
    expect(schema.brand.name).toBe("SKYCOT");
    expect(schema.applicationCategory).toBe("BusinessApplication");
    expect(schema.offers).toMatchObject({
      "@type": "AggregateOffer",
      priceCurrency: "USD",
      lowPrice: "0",
      highPrice: "29",
      offerCount: 1,
    });
  });
});

describe("faqPageSchema()", () => {
  it("wraps each entry as a Question with an accepted Answer", () => {
    const schema = faqPageSchema([
      { question: "Q1?", answer: "A1." },
      { question: "Q2?", answer: "A2." },
    ]);
    expect(schema["@type"]).toBe("FAQPage");
    expect(schema.mainEntity).toHaveLength(2);
    expect(schema.mainEntity[0]).toMatchObject({
      "@type": "Question",
      name: "Q1?",
      acceptedAnswer: { "@type": "Answer", text: "A1." },
    });
  });
});

describe("articleSchema()", () => {
  it("includes headline, dates, author, publisher, and keywords", () => {
    const schema = articleSchema({
      headline: "Test ship",
      datePublished: "2026-05-20",
      description: "What changed.",
      url: "https://skycot.com/changelog#v1.0.0",
      keywords: ["UX", "Brand"],
    });
    expect(schema["@type"]).toBe("Article");
    expect(schema.headline).toBe("Test ship");
    expect(schema.datePublished).toBe("2026-05-20");
    expect(schema.dateModified).toBe("2026-05-20");
    expect(schema.author).toMatchObject({ "@type": "Organization", name: "SKYCOT" });
    expect(schema.publisher).toMatchObject({ "@type": "Organization", name: "SKYCOT" });
    expect(schema.keywords).toEqual(["UX", "Brand"]);
    expect(schema.articleSection).toBe("Changelog");
  });
});
