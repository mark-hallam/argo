import type { Metadata } from "next";
import { notFound } from "next/navigation";
import Link from "next/link";
import { MarketingNav } from "@/components/marketing/MarketingNav";
import { MarketingFooter } from "@/components/marketing/MarketingFooter";
import { Markdown } from "@/app/_content/markdown";
import { loadPost } from "@/app/_content/posts";

interface Params {
  slug: string;
}

// Server-rendered per request. Was SSG via `generateStaticParams`, but the
// per-request nonce CSP (web/middleware.ts) cannot be baked into a
// prerendered post: its inline scripts would ship `nonce=""` and the
// runtime `'strict-dynamic'` policy would block every script on the page.
// `force-dynamic` lets each post pick up the request nonce so its scripts
// carry the matching value. (`generateStaticParams` removed: under
// force-dynamic it does not prevent prerendering reliably, and `loadPost`
// already 404s unknown slugs via notFound().)
export const dynamic = "force-dynamic";

export async function generateMetadata({
  params,
}: {
  params: Promise<Params>;
}): Promise<Metadata> {
  const { slug } = await params;
  const post = await loadPost(slug);
  if (!post) return {};
  return {
    title: post.title,
    description: post.summary,
    authors: [{ name: post.author }],
    openGraph: {
      type: "article",
      title: post.title,
      description: post.summary,
      url: `/blog/${post.slug}`,
      publishedTime: post.date,
      authors: [post.author],
    },
    alternates: { canonical: `/blog/${post.slug}` },
  };
}

export default async function BlogPostPage({
  params,
}: {
  params: Promise<Params>;
}) {
  const { slug } = await params;
  const post = await loadPost(slug);
  if (!post) notFound();
  const dateLabel = new Date(post.date).toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "BlogPosting",
    headline: post.title,
    datePublished: post.date,
    author: { "@type": "Person", name: post.author },
    description: post.summary,
  };
  return (
    <>
      <MarketingNav />
      <article className="mx-auto max-w-3xl px-6 py-16">
        <header className="mb-10 flex flex-col gap-3 border-b border-[var(--border)] pb-8">
          <div className="flex items-center gap-3 text-xs text-[var(--muted-foreground)]">
            <time dateTime={post.date}>{dateLabel}</time>
            <span aria-hidden>•</span>
            <span>{post.readingTimeMinutes} min read</span>
            <span aria-hidden>•</span>
            <span>{post.author}</span>
          </div>
          <h1 className="text-4xl sm:text-5xl font-bold tracking-tight text-[var(--foreground)]">
            {post.title}
          </h1>
          {post.summary ? (
            <p className="text-lg text-[var(--muted-foreground)] leading-relaxed">
              {post.summary}
            </p>
          ) : null}
        </header>
        <Markdown source={post.body} />
        <p className="mt-12 text-sm">
          <Link href="/blog" className="text-[var(--brand-primary)] underline">
            ← All posts
          </Link>
        </p>
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
        />
      </article>
      <MarketingFooter />
    </>
  );
}
