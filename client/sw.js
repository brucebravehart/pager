self.addEventListener('install', (e) => {
    console.log('Service Worker Installed');
});

self.addEventListener('fetch', (e) => {
    // Basic fetch handler to allow offline access later
    e.respondWith(fetch(e.request));
});

self.addEventListener('push', (event) => {
  const data = event.data ? event.data.json() : { title: 'New Message', body: 'You have a update!' };

  const options = {
    body: data.body,
    icon: '/images/icon.png',
    badge: '/images/badge.png', // Shown in the Android status bar
    vibrate: [100, 50, 100],
    data: { url: data.url }
  };

  event.waitUntil(
    self.registration.showNotification(data.title, options)
  );
});

// Open the PWA when the user clicks the notification
self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  event.waitUntil(
    clients.openWindow(event.notification.data.url || '/')
  );
});