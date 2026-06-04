import { describe, expect, it } from "vitest";
import { listPosts, loadPost } from "./posts";

// These read the co-located markdown bodies under `app/_content/blog/*.md`
// at test time the same way `next build` does (cwd = `web/`), so the suite
// doubles as a regression guard that the ported blog content stays parseable.

describe("blog posts loader (ported from marketing/)", () => {
  it("lists every ported post, newest first, with sane metadata", async () => {
    const posts = await listPosts();
    expect(posts.length).toBeGreaterThanOrEqual(3);
    for (const p of posts) {
      expect(p.slug).toMatch(/^[a-z0-9-]+$/);
      expect(p.title.length).toBeGreaterThan(0);
      expect(p.date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      expect(p.author.length).toBeGreaterThan(0);
      expect(p.readingTimeMinutes).toBeGreaterThanOrEqual(1);
    }
    const dates = posts.map((p) => p.date);
    expect([...dates].sort((a, b) => b.localeCompare(a))).toEqual(dates);
  });

  it("loads a known post body and returns null for an unknown slug", async () => {
    const post = await loadPost("why-we-built-skycot");
    expect(post).not.toBeNull();
    expect(post?.title).toBe("Why we built SKYCOT");
    expect(post?.body.length).toBeGreaterThan(0);
    expect(await loadPost("does-not-exist")).toBeNull();
  });
});
