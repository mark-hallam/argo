import type { MetadataRoute } from "next";

/**
 * `robots.txt` for crawlers. We allow indexing of the marketing
 * surfaces, block the (app) authed product + the /api routes, and
 * point at the dynamic sitemap.
 */
export default function robots(): MetadataRoute.Robots {
  const base = "https://skycot.com";
  return {
    rules: [
      {
        userAgent: "*",
        allow: ["/", "/landing", "/compare", "/pricing", "/customers", "/changelog", "/docs"],
        disallow: ["/api/", "/auth/", "/admin/"],
      },
    ],
    sitemap: `${base}/sitemap.xml`,
    host: base,
  };
}
