import { redirect } from "next/navigation";

/**
 * Worker profile index — redirects to the Overview tab.
 *
 * The freeze banner + freeze button now live on the surrounding
 * layout (see `./layout.tsx`) so they render on every tab, not
 * just the redirect-target index.
 */
export default async function WorkerIndexPage({
  params,
}: {
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = await params;
  redirect(`/workers/${workerId}/overview`);
}
