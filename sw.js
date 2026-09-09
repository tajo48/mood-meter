"use strict";

/* Offline support for the mood meter: the core assets are precached so the
   app works with no network, navigations serve the cached copy and refresh
   it in the background, and Google-Fonts requests are cached as they happen.
   All paths are relative, so the worker works at any URL depth. */

const VERSION = "1";
const CORE = "mood-core-" + VERSION;
const FONTS = "mood-fonts-" + VERSION;

const CORE_ASSETS = [
  "./",
  "./index.html",
  "./manifest.json",
  "./favicon.svg",
  "./favicon-32.png",
  "./apple-touch-icon.png",
  "./icon-192.png",
  "./icon-512.png",
  "./icon-192-maskable.png",
  "./icon-512-maskable.png",
];

self.addEventListener("install", (e) => {
  e.waitUntil(
    caches.open(CORE).then((cache) => cache.addAll(CORE_ASSETS)).then(() => self.skipWaiting())
  );
});

self.addEventListener("activate", (e) => {
  e.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(keys.filter((k) => k !== CORE && k !== FONTS).map((k) => caches.delete(k)))
      )
      .then(() => self.clients.claim())
  );
});

async function staleWhileRevalidate(request, cacheName) {
  const cache = await caches.open(cacheName);
  const cached = await cache.match(request, { ignoreSearch: true });
  const refreshed = fetch(request)
    .then((res) => {
      // "opaque" (no-cors) responses have status 0 but are fine to cache.
      if (res.ok || res.type === "opaque") cache.put(request, res.clone());
      return res;
    })
    .catch(() => null);
  return cached || (await refreshed) || Response.error();
}

self.addEventListener("fetch", (e) => {
  if (e.request.method !== "GET") return;
  const url = new URL(e.request.url);

  // App shell: serve cached instantly, refresh the cache in the background.
  if (e.request.mode === "navigate") {
    e.respondWith(
      (async () => {
        const cache = await caches.open(CORE);
        const cached =
          (await cache.match("./index.html", { ignoreSearch: true })) ||
          (await cache.match("./", { ignoreSearch: true }));
        const fresh = fetch(e.request)
          .then((res) => {
            if (res.ok) cache.put("./index.html", res.clone());
            return res;
          })
          .catch(() => null);
        return (
          cached ||
          (await fresh) ||
          new Response("offline", { status: 503, headers: { "Content-Type": "text/plain" } })
        );
      })()
    );
    return;
  }

  // Same-origin assets: cache-first, then network + cache fill-in.
  if (url.origin === self.location.origin) {
    e.respondWith(
      (async () => {
        const cache = await caches.open(CORE);
        const cached = await cache.match(e.request, { ignoreSearch: true });
        if (cached) return cached;
        try {
          const res = await fetch(e.request);
          if (res.ok) cache.put(e.request, res.clone());
          return res;
        } catch {
          return new Response("offline", { status: 503, headers: { "Content-Type": "text/plain" } });
        }
      })()
    );
    return;
  }

  // Fonts: stale-while-revalidate so the meter keeps its Bebas Neue offline.
  if (url.hostname === "fonts.googleapis.com" || url.hostname === "fonts.gstatic.com") {
    e.respondWith(staleWhileRevalidate(e.request, FONTS));
  }
});
