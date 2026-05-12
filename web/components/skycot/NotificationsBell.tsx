"use client";

import { Bell } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";

/**
 * Bell glyph + unread dot. Pulses briefly when the unread count
 * *increases* mid-session — a glanceable "1 new" cue for the
 * 60s-polling case. Kept lightweight (no popover deps) so the page
 * header can render it synchronously while the popover itself is
 * lazy-loaded.
 */
export function NotificationsBell({ unreadCount }: { unreadCount: number }) {
  const prevCountRef = useRef<number | null>(null);
  const [pulsing, setPulsing] = useState(false);

  useEffect(() => {
    const prev = prevCountRef.current;
    prevCountRef.current = unreadCount;
    if (prev != null && unreadCount > prev) {
      setPulsing(true);
      const t = setTimeout(() => setPulsing(false), 1600);
      return () => clearTimeout(t);
    }
  }, [unreadCount]);

  return (
    <span className="relative inline-flex">
      <Bell />
      {unreadCount > 0 ? (
        <span
          aria-label="Unread notifications"
          data-testid="notifications-unread-dot"
          data-pulse={pulsing ? "true" : undefined}
          className={cn(
            "absolute -right-0.5 -top-0.5 size-2 rounded-full bg-[var(--primary)] ring-2 ring-[var(--background)]",
            pulsing && "animate-ping",
          )}
        />
      ) : null}
    </span>
  );
}
