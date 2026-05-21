import { ogContentType, ogSize, renderOg } from "./_og/template";

export const runtime = "nodejs";
export const alt = "SKYCOT — Hire your first AI teammate";
export const size = ogSize;
export const contentType = ogContentType;

export default async function OpenGraphImage() {
  return renderOg({
    headline: "Hire your first AI teammate",
    pill: "Approval-first · Audit-first · Enterprise-grade",
  });
}
