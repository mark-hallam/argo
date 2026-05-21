import { ogContentType, ogSize, renderOg } from "../_og/template";

export const runtime = "nodejs";
export const alt = "SKYCOT vs Zapier, n8n, Crew.ai, Lindy";
export const size = ogSize;
export const contentType = ogContentType;

export default async function OpenGraphImage() {
  return renderOg({
    eyebrow: "Compare",
    headline: "SKYCOT vs Zapier, n8n, Crew.ai, Lindy",
    pill: "13 dimensions · 8 clean wins",
  });
}
