import { ogContentType, ogSize, renderOg } from "../_og/template";

export const runtime = "nodejs";
export const alt = "SKYCOT — Hire your first AI teammate";
export const size = ogSize;
export const contentType = ogContentType;

export default async function OpenGraphImage() {
  return renderOg({
    eyebrow: "Product",
    headline: "Don't buy another tool. Hire a teammate.",
    pill: "Lives in Slack · Asks before acting · Earns trust",
  });
}
