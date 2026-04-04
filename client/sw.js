const CACHE_VERSION = 'v2';
const CACHE_NAME = `pager-static-${CACHE_VERSION}`;
const API_PATHS = ['/status', '/users', '/register_user', '/send-push'];
// Add every file your app needs to run (HTML, CSS, JS, Images)
const ASSETS_TO_CACHE = [
  './',
  'index.html',
  'style.css',
  'script.js',
  'alert_small.png',
  'alert_small_bw.png',
  'manifest.json',
  'favicon.ico',
  'icon-192.png',
  'icon-512.png'
];

// 1. INSTALL: Save the files locally
self.addEventListener('install', (event) => {
  console.log('SW: Installing and caching assets');
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(ASSETS_TO_CACHE);
    })
  );
  // Forces the waiting service worker to become the active service worker
  self.skipWaiting();
});

// 2. ACTIVATE: Clean up old caches if you update the version
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames.map((cache) => {
          if (cache !== CACHE_NAME) {
            console.log('SW: Clearing old cache');
            return caches.delete(cache);
          }
        })
      );
    })
  );

  // Start controlling open pages right away after activation.
  self.clients.claim();
});

// 3. FETCH: The logic to handle offline requests
self.addEventListener('fetch', (event) => {
  if (event.request.method !== 'GET') {
    return;
  }

  const requestUrl = new URL(event.request.url);

  // Do not cache API traffic; always hit network for fresh button actions.
  if (API_PATHS.includes(requestUrl.pathname)) {
    return;
  }

  // Only handle same-origin static/page requests.
  if (requestUrl.origin !== self.location.origin) {
    return;
  }

  event.respondWith(
    // Check the cache first
    caches.match(event.request).then((cachedResponse) => {
      if (cachedResponse) {
        return cachedResponse; // Return the local version
      }

      // If not in cache, try the network
      return fetch(event.request).then((networkResponse) => {
        // Cache successful same-origin GET requests for future loads.
        if (networkResponse && networkResponse.ok && requestUrl.origin === self.location.origin) {
          const responseToCache = networkResponse.clone();
          caches.open(CACHE_NAME).then((cache) => {
            cache.put(event.request, responseToCache);
          });
        }
        return networkResponse;
      }).catch(() => {
        // If network fails and it's a page navigation, you can return a fallback
        if (event.request.mode === 'navigate') {
          return caches.match('/index.html');
        }
      });
    })
  );
});

// --- Keep your Push and Notification logic below ---
self.addEventListener('push', (event) => {
  const data = event.data ? event.data.json() : { title: 'New Message', body: 'You have an update!' };
  const options = {
    body: data.body,
    icon: 'alert_small.png',
    badge: 'alert_small_bw.png', // Shown in the Android status bar
    vibrate: [100, 50, 100],
    data: { url: data.url }
  };
  event.waitUntil(self.registration.showNotification(data.title, options));
});

self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  event.waitUntil(clients.openWindow(event.notification.data.url || '/'));
});