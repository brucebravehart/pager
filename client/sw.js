const CACHE_VERSION = 'v2';
const CACHE_NAME = `pager-static-${CACHE_VERSION}`;
const API_PATHS = ['/status', '/users', '/register_user', '/send-push'];
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

self.addEventListener('install', (event) => {
  console.log('SW: Installing and caching assets');
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(ASSETS_TO_CACHE);
    })
  );
  self.skipWaiting();
});

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

  self.clients.claim();
});

self.addEventListener('fetch', (event) => {
  if (event.request.method !== 'GET') {
    return;
  }

  const requestUrl = new URL(event.request.url);

  if (API_PATHS.includes(requestUrl.pathname)) {
    return;
  }

  if (requestUrl.origin !== self.location.origin) {
    return;
  }

  event.respondWith(
    caches.match(event.request).then((cachedResponse) => {
      if (cachedResponse) {
        return cachedResponse;
      }

      return fetch(event.request).then((networkResponse) => {
        if (networkResponse && networkResponse.ok && requestUrl.origin === self.location.origin) {
          const responseToCache = networkResponse.clone();
          caches.open(CACHE_NAME).then((cache) => {
            cache.put(event.request, responseToCache);
          });
        }
        return networkResponse;
      }).catch(() => {
        if (event.request.mode === 'navigate') {
          return caches.match('/index.html');
        }
      });
    })
  );
});

self.addEventListener('push', (event) => {
  const data = event.data ? event.data.json() : { title: 'New Message', body: 'You have an update!' };
  const options = {
    body: data.body,
    icon: 'alert_small.png',
    badge: 'alert_small_bw.png',
    vibrate: [100, 50, 100],
    data: { url: data.url }
  };
  event.waitUntil(self.registration.showNotification(data.title, options));
});

self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  event.waitUntil(clients.openWindow(event.notification.data.url || '/'));
});