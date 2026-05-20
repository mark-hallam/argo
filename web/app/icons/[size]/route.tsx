import { ImageResponse } from "next/og";

export const dynamic = "force-static";

const ALLOWED = new Map<string, { size: number; maskable: boolean }>([
  ["icon-192", { size: 192, maskable: false }],
  ["icon-256", { size: 256, maskable: false }],
  ["icon-384", { size: 384, maskable: false }],
  ["icon-512", { size: 512, maskable: false }],
  ["icon-512-maskable", { size: 512, maskable: true }],
]);

export async function generateStaticParams() {
  return Array.from(ALLOWED.keys()).map((size) => ({ size }));
}

export async function GET(
  _req: Request,
  { params }: { params: Promise<{ size: string }> },
) {
  const { size: slug } = await params;
  const cfg = ALLOWED.get(slug);
  if (!cfg) {
    return new Response("not found", { status: 404 });
  }

  const { size, maskable } = cfg;
  // Maskable icons need ~10% safe-area padding so platforms can crop to any shape.
  const innerScale = maskable ? 0.5 : 0.62;
  const diamond = Math.round(size * innerScale);
  const radius = Math.round(size * (maskable ? 0 : 0.18));

  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background:
            "linear-gradient(135deg, #8B5CF6 0%, #A855F7 50%, #EC4899 100%)",
          borderRadius: radius,
        }}
      >
        <div
          style={{
            width: diamond,
            height: diamond,
            background: "#FEF7F0",
            transform: "rotate(45deg)",
            borderRadius: Math.round(diamond * 0.06),
          }}
        />
      </div>
    ),
    { width: size, height: size },
  );
}
