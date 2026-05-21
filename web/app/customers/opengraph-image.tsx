import { ogContentType, ogSize, renderOg } from "../_og/template";

export const runtime = "nodejs";
export const alt = "SKYCOT Customers — 300+ teams shipping with AI teammates";
export const size = ogSize;
export const contentType = ogContentType;

export default async function OpenGraphImage() {
  return renderOg({
    eyebrow: "Customers",
    headline: "300+ teams shipping with AI teammates",
    pill: "From solo founders to Fortune 500",
  });
}
