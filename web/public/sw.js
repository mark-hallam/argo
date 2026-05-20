// SKYCOT PWA service worker.
// Strategy: network-first for API, cache-first for static assets,
// offline shell fallback for navigations.
// Bump CACHE_VERSION to invalidate clients after deploys that change the shell.

const CACHE_VERSION = "skycot-pwa-v1";
const SHELL_CACHE = `${CACHE_VERSION}-shell`;
const RUNTIME_CACHE = `${CACHE_VERSION}-runtime`;
const OFFLINE_URL = "/offline";

const PRECACHE_URLS = [
  OFFLINE_URL,
  "/manifest.webmanifest",
  "/favicon/favicon.svg",
  "/brand/skycot-wordmark.svg",
  "/icons/icon-192",
  "/icons/icon-512",
];

self.addEventListener("install", (event) => {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(SHELL_CACHE);
      // Use addAll-with-tolerance: if any one asset 404s in dev, the whole
      // install would otherwise fail.
      await Promise.all(
        PRECACHE_URLS.map(async (url) => {
          try {
            const response = await fetch(url, { cache: "reload" });
            if (response.ok) await cache.put(url, response);
          } catch (_) {
            /* ignore individual failures */
          }
        }),
      );
      await self.skipWaiting();
    })(),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const keys = await caches.keys();
      await Promise.all(
        keys
          .filter((k) => !k.startsWith(CACHE_VERSION))
          .map((k) => caches.delete(k)),
      );
      await self.clients.claim();
    })(),
  );
});

function isApiRequest(url) {
  return url.pathname.startsWith("/api/");
}

function isStaticAsset(url) {
  return (
    url.pathname.startsWith("/_next/static/") ||
    url.pathname.startsWith("/icons/") ||
    url.pathname.startsWith("/brand/") ||
    url.pathname.startsWith("/favicon/") ||
    url.pathname === "/manifest.webmanifest"
  );
}

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET") return;

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;

  // Network-first for API. Don't cache — fresh auth & data matter more.
  if (isApiRequest(url)) {
    event.respondWith(fetch(request).catch(() => Response.error()));
    return;
  }

  // Cache-first for static assets.
  if (isStaticAsset(url)) {
    event.respondWith(
      (async () => {
        const cached = await caches.match(request);
        if (cached) return cached;
        try {
          const response = await fetch(request);
          if (response.ok) {
            const cache = await caches.open(RUNTIME_CACHE);
            cache.put(request, response.clone());
          }
          return response;
        } catch (_) {
          return Response.error();
        }
      })(),
    );
    return;
  }

  // Navigation requests: network-first, fall back to offline shell.
  if (request.mode === "navigate") {
    event.respondWith(
      (async () => {
        try {
          const response = await fetch(request);
          return response;
        } catch (_) {
          const cached = await caches.match(OFFLINE_URL);
          return cached || new Response("Offline", { status: 503 });
        }
      })(),
    );
  }
});

// Allow the page to trigger immediate activation after an update.
self.addEventListener("message", (event) => {
  if (event.data === "SKIP_WAITING") {
    self.skipWaiting();
  }
});
