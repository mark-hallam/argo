import { ogContentType, ogSize, renderOg } from "../_og/template";

export const runtime = "nodejs";
export const alt = "SKYCOT Pricing — Hire teammates. Pay like they're real.";
export const size = ogSize;
export const contentType = ogContentType;

export default async function OpenGraphImage() {
  return renderOg({
    eyebrow: "Pricing",
    headline: "Hire teammates. Pay like they're real.",
    pill: "Free to start · Pro · Enterprise",
  });
}
