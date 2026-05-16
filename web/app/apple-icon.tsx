import { ImageResponse } from "next/og";

export const size = { width: 180, height: 180 };
export const contentType = "image/png";

export default function AppleIcon() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background: "oklch(0.55 0.18 256)",
          borderRadius: 36,
        }}
      >
        <div
          style={{
            width: 80,
            height: 80,
            background: "oklch(0.99 0.005 256)",
            transform: "rotate(45deg)",
          }}
        />
      </div>
    ),
    { ...size },
  );
}
