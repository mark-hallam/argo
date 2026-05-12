import { redirect } from "next/navigation";

/**
 * Default landing for `/workers/<id>` — redirect to the Overview tab
 * so the URL always carries the tab segment. Tabs are server
 * navigations; this redirect runs on the server and avoids the
 * flash-of-bare-hero.
 */
export default async function WorkerIndexPage({
  params,
}: {
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = await params;
  redirect(`/workers/${workerId}/overview`);
}
