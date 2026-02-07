self.addEventListener('install', (e) => {
    console.log('Service Worker Installed');
});

self.addEventListener('fetch', (e) => {
    // Basic fetch handler to allow offline access later
    e.respondWith(fetch(e.request));
});