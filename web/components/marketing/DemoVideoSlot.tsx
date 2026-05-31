"use client";

import { useState } from "react";
import { Play, Clock } from "lucide-react";

/**
 * Demo-video slot for the public `/landing` page.
 *
 * ⚠️ PLACEHOLDER — the v1.0 demo MP4 (`docs/skycot/demos/v1-mission-mvp.mp4`)
 * is an operator-only artifact that HAS NOT BEEN RECORDED yet (it's the v1.0
 * ship gate). This component renders a clean, branded poster frame so the
 * operator can drop the file in later by passing `src` — until then it shows
 * a labeled "coming soon" poster and does NOT pretend a video exists.
 *
 * When the MP4 is ready: drop it at `web/public/demo/v1-mission-mvp.mp4` (+
 * an optional `poster.jpg`) and pass `src="/demo/v1-mission-mvp.mp4"`. The
 * poster click then swaps to a native `<video controls autoPlay>`.
 */

export interface DemoVideoSlotProps {
  /** Public path to the MP4 once recorded, e.g. "/demo/v1-mission-mvp.mp4". */
  src?: string;
  /** Optional poster image path. */
  poster?: string;
  /** Stated runtime label, e.g. "90-second tour". */
  durationLabel?: string;
}

export function DemoVideoSlot({
  src,
  poster,
  durationLabel = "90-second tour",
}: DemoVideoSlotProps) {
  const [playing, setPlaying] = useState(false);

  if (src && playing) {
    return (
      <div className="demo-frame" data-testid="demo-video-playing">
        {/* eslint-disable-next-line jsx-a11y/media-has-caption */}
        <video
          className="demo-video"
          src={src}
          poster={poster}
          controls
          autoPlay
          playsInline
        />
      </div>
    );
  }

  return (
    <div
      className={`demo-frame demo-poster ${src ? "is-ready" : "is-pending"}`}
      data-testid="demo-video-slot"
      role={src ? "button" : undefined}
      tabIndex={src ? 0 : undefined}
      onClick={src ? () => setPlaying(true) : undefined}
      onKeyDown={
        src
          ? (e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                setPlaying(true);
              }
            }
          : undefined
      }
      aria-label={src ? "Play the SKYCOT product demo" : "Product demo — coming soon"}
    >
      <div className="demo-poster-art" aria-hidden="true">
        <div className="demo-grid-bg" />
        <div className="demo-play-btn">
          <Play size={26} fill="currentColor" />
        </div>
      </div>
      <div className="demo-poster-meta">
        {src ? (
          <span className="demo-meta-pill demo-meta-ready">
            <Play size={12} /> Watch the demo · {durationLabel}
          </span>
        ) : (
          <span className="demo-meta-pill demo-meta-pending" data-testid="demo-pending-pill">
            <Clock size={12} /> Demo film · recording now · {durationLabel}
          </span>
        )}
      </div>
    </div>
  );
}
