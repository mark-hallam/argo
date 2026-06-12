"use client";

import { useState } from "react";
import Link from "next/link";
import { Play, ArrowRight } from "lucide-react";

/**
 * Demo slot for the public `/landing` page.
 *
 * ⚠️ The v1.0 demo MP4 (`docs/skycot/demos/v1-mission-mvp.mp4`) is an
 * operator-only artifact that HAS NOT BEEN RECORDED yet (it's the v1.0 ship
 * gate). Rather than render a play button that does nothing (a dead handler —
 * NO THEATER), the slot points visitors at the LIVE demo workspace journey
 * (`/auth/signup` → demo workspace, no email required) until the film exists.
 *
 * When the MP4 is ready: drop it at `web/public/demo/v1-mission-mvp.mp4` (+
 * an optional `poster.jpg`) and pass `src="/demo/v1-mission-mvp.mp4"`. The
 * poster click then swaps to a native `<video controls autoPlay>` — a
 * one-line change, no markup churn.
 */

export interface DemoVideoSlotProps {
  /** Public path to the MP4 once recorded, e.g. "/demo/v1-mission-mvp.mp4". */
  src?: string;
  /** Optional poster image path. */
  poster?: string;
  /** Stated runtime label, e.g. "90-second tour". */
  durationLabel?: string;
  /**
   * Where the live-demo CTA points while no `src` is configured.
   * Defaults to the email-less demo-workspace journey.
   */
  liveHref?: string;
}

export function DemoVideoSlot({
  src,
  poster,
  durationLabel = "90-second tour",
  liveHref = "/auth/signup",
}: DemoVideoSlotProps) {
  const [playing, setPlaying] = useState(false);

  // Recorded film exists and the viewer pressed play: real native video.
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

  // Recorded film exists, not yet playing: poster acts as a real play button.
  if (src) {
    return (
      <div
        className="demo-frame demo-poster is-ready"
        data-testid="demo-video-slot"
        role="button"
        tabIndex={0}
        onClick={() => setPlaying(true)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            setPlaying(true);
          }
        }}
        aria-label="Play the SKYCOT product demo"
      >
        <div className="demo-poster-art" aria-hidden="true">
          <div className="demo-grid-bg" />
          <div className="demo-play-btn">
            <Play size={26} fill="currentColor" />
          </div>
        </div>
        <div className="demo-poster-meta">
          <span className="demo-meta-pill demo-meta-ready">
            <Play size={12} /> Watch the demo · {durationLabel}
          </span>
        </div>
      </div>
    );
  }

  // No film yet: the CTA is a REAL link into the live demo workspace, not a
  // dead play button. Clicking it navigates — no fake video, no mock autoplay.
  return (
    <Link
      href={liveHref}
      className="demo-frame demo-poster is-live-cta"
      data-testid="demo-video-slot"
      aria-label="Try SKYCOT live in a demo workspace — 5 minutes, no email"
    >
      <div className="demo-poster-art" aria-hidden="true">
        <div className="demo-grid-bg" />
        <div className="demo-play-btn">
          <Play size={26} fill="currentColor" />
        </div>
      </div>
      <div className="demo-poster-meta">
        <span className="demo-meta-pill demo-meta-live" data-testid="demo-live-cta-pill">
          Try it live — 5 minutes, no email <ArrowRight size={12} />
        </span>
      </div>
    </Link>
  );
}
