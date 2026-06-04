import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

/**
 * Blog post loader for the ported `/blog` route. Reads the co-located
 * markdown bodies under `web/app/_content/blog/*.md` at build time (the
 * blog pages are statically generated, so these reads happen during
 * `next build`, not per-request). Frontmatter is parsed by hand and
 * reading time is estimated inline so `web/` needs no new dependency
 * (the old `marketing/` app pulled in `reading-time` + `@next/mdx`;
 * neither came across in the consolidation).
 */
export interface PostMeta {
  slug: string;
  title: string;
  summary: string;
  date: string;
  author: string;
  readingTimeMinutes: number;
}

export interface Post extends PostMeta {
  body: string;
}

const FRONTMATTER_RE = /^---\s*\n([\s\S]*?)\n---\s*\n([\s\S]*)$/;

function parseFrontmatter(raw: string): {
  data: Record<string, string>;
  body: string;
} {
  const match = raw.match(FRONTMATTER_RE);
  if (!match) return { data: {}, body: raw };
  const data: Record<string, string> = {};
  for (const line of match[1].split("\n")) {
    const idx = line.indexOf(":");
    if (idx <= 0) continue;
    const key = line.slice(0, idx).trim();
    let value = line.slice(idx + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    data[key] = value;
  }
  return { data, body: match[2] };
}

/** ~200 words/min reading estimate; floor of 1 minute. */
function estimateReadingMinutes(body: string): number {
  const words = body.trim().split(/\s+/).filter(Boolean).length;
  return Math.max(1, Math.round(words / 200));
}

const CONTENT_DIR = path.join(process.cwd(), "app", "_content", "blog");

export async function loadPost(slug: string): Promise<Post | null> {
  try {
    const raw = await readFile(path.join(CONTENT_DIR, `${slug}.md`), "utf8");
    const { data, body } = parseFrontmatter(raw);
    if (!data.title || !data.date) return null;
    return {
      slug,
      title: data.title,
      summary: data.summary ?? "",
      date: data.date,
      author: data.author ?? "SKYCOT",
      readingTimeMinutes: estimateReadingMinutes(body),
      body,
    };
  } catch {
    return null;
  }
}

export async function listPosts(): Promise<PostMeta[]> {
  const entries = await readdir(CONTENT_DIR);
  const slugs = entries
    .filter((f) => f.endsWith(".md"))
    .map((f) => f.replace(/\.md$/, ""));
  const posts = await Promise.all(slugs.map(loadPost));
  return posts
    .filter((p): p is Post => p !== null)
    .sort((a, b) => b.date.localeCompare(a.date))
    .map(({ body: _body, ...meta }) => meta);
}
