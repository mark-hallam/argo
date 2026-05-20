"use client";

import { useCallback } from "react";

export default function OfflinePage() {
  const onRetry = useCallback(() => {
    if (typeof window !== "undefined") window.location.reload();
  }, []);

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <main className="offline-shell" data-testid="offline-shell">
        <div className="offline-blob offline-blob-a" aria-hidden="true" />
        <div className="offline-blob offline-blob-b" aria-hidden="true" />
        <div className="offline-stage">
          <div className="offline-mark" aria-hidden="true" />
          <p className="offline-eyebrow">SKYCOT</p>
          <h1 className="offline-hero">You&rsquo;re offline.</h1>
          <p className="offline-body">
            SKYCOT will reconnect automatically when you&rsquo;re back online. Your last loaded
            screen stays open in the meantime.
          </p>
          <button type="button" onClick={onRetry} className="offline-cta">
            Try again
          </button>
        </div>
        <style jsx global>{`
          html,
          body {
            background: #fef7f0;
          }
          .offline-shell {
            position: relative;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 32px 24px;
            font-family: "Inter", sans-serif;
            color: #1e1b4b;
            overflow: hidden;
            background: #fef7f0;
          }
          .offline-blob {
            position: absolute;
            border-radius: 50%;
            filter: blur(60px);
            pointer-events: none;
            z-index: 0;
          }
          .offline-blob-a {
            top: -20%;
            right: -10%;
            width: 540px;
            height: 540px;
            background: radial-gradient(circle, rgba(139, 92, 246, 0.22) 0%, transparent 70%);
          }
          .offline-blob-b {
            bottom: -25%;
            left: -10%;
            width: 620px;
            height: 620px;
            background: radial-gradient(circle, rgba(236, 72, 153, 0.18) 0%, transparent 70%);
          }
          .offline-stage {
            position: relative;
            z-index: 1;
            max-width: 520px;
            text-align: center;
          }
          .offline-mark {
            width: 56px;
            height: 56px;
            margin: 0 auto 20px;
            border-radius: 14px;
            background: linear-gradient(135deg, #8b5cf6, #ec4899);
            box-shadow: 0 12px 30px -8px rgba(139, 92, 246, 0.5);
          }
          .offline-eyebrow {
            font-family: "Audiowide", sans-serif;
            font-size: 14px;
            letter-spacing: 0.18em;
            color: #6366f1;
            margin: 0 0 16px;
          }
          .offline-hero {
            font-family: "Audiowide", sans-serif;
            font-size: clamp(40px, 8vw, 64px);
            line-height: 1.05;
            margin: 0 0 18px;
            background: linear-gradient(135deg, #8b5cf6, #ec4899);
            -webkit-background-clip: text;
            background-clip: text;
            color: transparent;
          }
          .offline-body {
            font-size: 17px;
            line-height: 1.55;
            color: #3f3f46;
            margin: 0 0 28px;
          }
          .offline-cta {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            padding: 11px 22px;
            border-radius: 999px;
            border: 0;
            background: linear-gradient(135deg, #8b5cf6, #ec4899);
            color: white;
            font-family: "Inter", sans-serif;
            font-size: 15px;
            font-weight: 700;
            cursor: pointer;
            box-shadow: 0 8px 24px -4px rgba(139, 92, 246, 0.5);
          }
          .offline-cta:hover {
            transform: translateY(-1px);
          }
        `}</style>
      </main>
    </>
  );
}
