import type { MetadataRoute } from "next";

/**
 * `robots.txt` for crawlers. We allow indexing of the marketing
 * surfaces, block the (app) authed product + the /api routes, and
 * point at the dynamic sitemap.
 *
 * `/customers` (the index logo gallery) stays indexable so SEO has
 * the marquee; the per-customer detail pages under `/customers/<slug>`
 * are gated until they describe real customers.
 */
export default function robots(): MetadataRoute.Robots {
  const base = "https://skycot.com";
  return {
    rules: [
      {
        userAgent: "*",
        allow: ["/", "/landing", "/compare", "/pricing", "/customers", "/team", "/changelog", "/docs"],
        disallow: ["/api/", "/auth/", "/admin/", "/customers/"],
      },
    ],
    sitemap: `${base}/sitemap.xml`,
    host: base,
  };
}
