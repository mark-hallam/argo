import { ogContentType, ogSize, renderOg } from "../_og/template";

export const runtime = "nodejs";
export const alt = "SKYCOT Customers — representative deployment profiles";
export const size = ogSize;
export const contentType = ogContentType;

export default async function OpenGraphImage() {
  return renderOg({
    eyebrow: "Representative profiles",
    headline: "Teams SKYCOT is built to support",
    pill: "Design partners onboarding",
  });
}
