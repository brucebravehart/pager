document.addEventListener('DOMContentLoaded', () => {
    const onboarding = document.getElementById('onboarding');
    const home = document.getElementById('home');
    const nameInput = document.getElementById('userNameInput');
    const displayName = document.getElementById('displayName');
    const isIOS = /iPad|iPhone|iPod/.test(navigator.userAgent) && !window.MSStream;
    const isStandalone = window.matchMedia('(display-mode: standalone)').matches || window.navigator.standalone;

    // 1. Check if user already exists
    const savedName = localStorage.getItem('pwa_user_name');

    if (savedName) {
        showHomeScreen(savedName);
    } else {
        onboarding.classList.remove('hidden');
    }

    // 2. Handle Onboarding
    document.getElementById('getStartedBtn').addEventListener('click', async () => {
        const name = nameInput.value.trim();
        if (!name) return alert("Please enter a name");

        if (isIOS && !isStandalone) {
            // iOS Workaround: You cannot ask for push permission in Safari tabs
            alert("To enable notifications on iOS: \n1. Tap the 'Share' icon \n2. Select 'Add to Home Screen' \n3. Open the app from your Home Screen");
            return;
        }

        // Request Push Permission
        try {
            const permission = await Notification.requestPermission();
            if (permission === 'granted') {
                localStorage.setItem('pwa_user_name', name);
                
                // Register for Push
                await subscribeUserToPush();
                showHomeScreen(name);
            } else {
                alert("Permission denied. We need notifications to work!");
            }
        } catch (err) {
            console.error("Push registration failed:", err);
        }
    });

    function showHomeScreen(name) {
        onboarding.classList.add('hidden');
        home.classList.remove('hidden');
        displayName.textContent = name;
    }

    // Home Screen Actions
    document.getElementById('actionBtn').addEventListener('click', () => {
        const li = document.createElement('li');
        li.textContent = `Entry created at ${new Date().toLocaleTimeString()}`;
        document.getElementById('itemList').appendChild(li);
    });

    document.getElementById('resetBtn').addEventListener('click', () => {
        localStorage.clear();
        location.reload();
    });
});

async function subscribeUserToPush() {
    const registration = await navigator.serviceWorker.ready;
    const subscription = await registration.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: urlBase64ToUint8Array(VAPID_PUBLIC_KEY)
    });

    // Send this subscription object to your server to store it
    console.log("Subscription Object:", JSON.stringify(subscription));
    // await fetch('/save-subscription', { method: 'POST', body: JSON.stringify(subscription) });
}

function urlBase64ToUint8Array(base64String) {
    const padding = '='.repeat((4 - base64String.length % 4) % 4);
    const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
    const rawData = window.atob(base64);
    return Uint8Array.from([...rawData].map((char) => char.charCodeAt(0)));
}

// Register Service Worker
if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('sw.js');
}