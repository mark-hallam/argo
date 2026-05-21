import { ImageResponse } from "next/og";

export const ogSize = { width: 1200, height: 630 };
export const ogContentType = "image/png";

export async function loadAudiowide(): Promise<ArrayBuffer> {
  const css = await fetch(
    "https://fonts.googleapis.com/css2?family=Audiowide&display=swap",
    { headers: { "User-Agent": "Mozilla/5.0" } },
  ).then((r) => r.text());
  const match = css.match(/src: url\(([^)]+)\) format\('truetype'\)/);
  if (!match) {
    throw new Error("Audiowide font URL not found in Google Fonts CSS");
  }
  return fetch(match[1]).then((r) => r.arrayBuffer());
}

export type OgVariant = {
  eyebrow?: string;
  headline: string;
  pill: string;
};

export async function renderOg(variant: OgVariant): Promise<ImageResponse> {
  const audiowide = await loadAudiowide();

  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          background:
            "linear-gradient(135deg, #4c1d95 0%, #8B5CF6 45%, #EC4899 100%)",
          position: "relative",
        }}
      >
        <div
          style={{
            position: "absolute",
            top: -200,
            right: -200,
            width: 600,
            height: 600,
            borderRadius: 9999,
            background:
              "radial-gradient(circle, rgba(236, 72, 153, 0.45) 0%, transparent 70%)",
          }}
        />
        <div
          style={{
            position: "absolute",
            bottom: -180,
            left: -180,
            width: 520,
            height: 520,
            borderRadius: 9999,
            background:
              "radial-gradient(circle, rgba(139, 92, 246, 0.5) 0%, transparent 70%)",
          }}
        />
        {variant.eyebrow ? (
          <div
            style={{
              fontSize: 26,
              color: "#fce7f3",
              letterSpacing: 6,
              textTransform: "uppercase",
              fontWeight: 600,
              marginBottom: 28,
            }}
          >
            {variant.eyebrow}
          </div>
        ) : null}
        <div
          style={{
            fontFamily: "Audiowide",
            fontSize: 140,
            color: "#ffffff",
            letterSpacing: -1,
            lineHeight: 1,
            textShadow: "0 6px 32px rgba(15, 23, 42, 0.35)",
          }}
        >
          SKYCOT
        </div>
        <div
          style={{
            marginTop: 30,
            fontSize: 48,
            color: "#f5f3ff",
            fontWeight: 500,
            letterSpacing: -0.5,
            textAlign: "center",
            maxWidth: 1000,
            lineHeight: 1.15,
            paddingLeft: 60,
            paddingRight: 60,
          }}
        >
          {variant.headline}
        </div>
        <div
          style={{
            marginTop: 44,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            padding: "16px 32px",
            background: "rgba(255, 255, 255, 0.14)",
            border: "2px solid rgba(255, 255, 255, 0.35)",
            borderRadius: 999,
            color: "#ffffff",
            fontSize: 22,
            fontWeight: 500,
            letterSpacing: 0.2,
          }}
        >
          {variant.pill}
        </div>
      </div>
    ),
    {
      ...ogSize,
      fonts: [
        {
          name: "Audiowide",
          data: audiowide,
          style: "normal",
          weight: 400,
        },
      ],
    },
  );
}
