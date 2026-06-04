import Link from "next/link";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";
import { listPosts, type PostMeta } from "@/app/_content/posts";

// Statically generated at build time from the co-located markdown.
export const dynamic = "force-static";

function PostCard({ post }: { post: PostMeta }) {
  const dateLabel = new Date(post.date).toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
  return (
    <article className="group rounded-2xl border border-[var(--border)] bg-[var(--card)] p-6 transition-colors hover:border-[var(--brand-primary)]/40">
      <Link
        href={{ pathname: `/blog/${post.slug}` }}
        className="flex flex-col gap-3"
      >
        <header className="flex flex-col gap-2">
          <div className="flex items-center gap-3 text-xs text-[var(--muted-foreground)]">
            <time dateTime={post.date}>{dateLabel}</time>
            <span aria-hidden>•</span>
            <span>{post.readingTimeMinutes} min read</span>
          </div>
          <h2 className="text-xl font-semibold text-[var(--foreground)] group-hover:text-[var(--brand-primary)]">
            {post.title}
          </h2>
        </header>
        <p className="text-sm leading-relaxed text-[var(--muted-foreground)]">
          {post.summary}
        </p>
        <footer className="text-xs text-[var(--muted-foreground)]">
          By {post.author}
        </footer>
      </Link>
    </article>
  );
}

export default async function BlogIndexPage() {
  const posts = await listPosts();
  return (
    <>
      <MarketingNav />
      <section className="w-full px-6 py-16 sm:py-24 text-center">
        <div className="mx-auto max-w-3xl flex flex-col items-center gap-5">
          <p className="text-sm font-medium tracking-wide text-[var(--brand-primary)] uppercase">
            Blog
          </p>
          <h1 className="text-4xl sm:text-5xl md:text-6xl font-bold tracking-tight text-[var(--foreground)] leading-tight">
            Notes from the swarm
          </h1>
          <p className="max-w-2xl text-base sm:text-lg text-[var(--muted-foreground)] leading-relaxed">
            Shipping AI workers, computer-use on the desktop, and how we built
            SKYCOT — written by the people doing the work.
          </p>
        </div>
      </section>
      <section className="mx-auto max-w-4xl px-6 pb-24">
        {posts.length === 0 ? (
          <p className="text-sm text-[var(--muted-foreground)]">
            No posts yet. Check back soon.
          </p>
        ) : (
          <ul className="grid gap-6">
            {posts.map((post) => (
              <li key={post.slug}>
                <PostCard post={post} />
              </li>
            ))}
          </ul>
        )}
        <p className="mt-12 text-xs text-[var(--muted-foreground)]">
          Subscribe via{" "}
          <a className="underline" href="/blog/feed.xml">
            RSS
          </a>
          .
        </p>
      </section>
      <MarketingFooter />
    </>
  );
}
