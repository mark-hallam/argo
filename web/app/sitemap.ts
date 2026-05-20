import type { MetadataRoute } from "next";

/**
 * Sitemap for crawlers. Listed: every public marketing surface that
 * Google/Bing/Bingbot should index. Excluded: any `(app)` group route
 * (auth-gated), `/auth/*` flows (transient), `/api/*` (machine-only).
 */
export default function sitemap(): MetadataRoute.Sitemap {
  const base = "https://skycot.com";
  const now = new Date();
  return [
    { url: `${base}/`, lastModified: now, changeFrequency: "weekly", priority: 1 },
    { url: `${base}/landing`, lastModified: now, changeFrequency: "weekly", priority: 1 },
    { url: `${base}/compare`, lastModified: now, changeFrequency: "weekly", priority: 0.9 },
    { url: `${base}/pricing`, lastModified: now, changeFrequency: "weekly", priority: 0.9 },
    { url: `${base}/customers`, lastModified: now, changeFrequency: "weekly", priority: 0.8 },
    { url: `${base}/changelog`, lastModified: now, changeFrequency: "daily", priority: 0.7 },
    { url: `${base}/docs`, lastModified: now, changeFrequency: "weekly", priority: 0.7 },
  ];
}
