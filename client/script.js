const VAPID_PUBLIC_KEY = "BHONvdPYBAF8hLiQZjImtAz74WpIa2fwwLdzr7CUAyIIQW-f8SPHg-JZGyOwDrXjHWfj4PPIvpeMZjr5HRl29BU"; // Get this from your backend
const BACKEND_URL = "https://pager-87gw.onrender.com:443"

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
        showHomeScreen(JSON.parse(savedName).name);
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
                
                // delete old subscription
                const registration = await navigator.serviceWorker.ready;
                const existingSubscription = await registration.pushManager.getSubscription();
                if (existingSubscription) {
                    await existingSubscription.unsubscribe();
                    console.log("Old subscription cleared.");
                }
                // Register for Push
                const subscription = await subscribeUserToPush();

                const subscriptionJson = {
                    endpoint: subscription.endpoint,
                    // Convert binary ArrayBuffers to Base64 strings
                    p256dh: btoa(String.fromCharCode(...new Uint8Array(subscription.getKey('p256dh')))),
                    auth: btoa(String.fromCharCode(...new Uint8Array(subscription.getKey('auth'))))
                };

                
                const response = await fetch(BACKEND_URL + '/register_user', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({'name': name, 'subObj': subscriptionJson})
                })

                const response_json = await response.json()

                console.log(response, response_json)

                localStorage.setItem('pwa_user_name', JSON.stringify({'name': name, 'subObj': subscriptionJson}));

                showHomeScreen(name);
            } else {
                alert("Permission denied. We need notifications to work!");
            }
        } catch (err) {
            alert("Push registration failed")
            console.error("Push registration failed:", err);
        }

        // TODO: debug
        showHomeScreen(name);
    });

    function showHomeScreen(name) {
        onboarding.classList.add('hidden');
        home.classList.remove('hidden');
        displayName.textContent = name;
        renderUserList()
    }

    async function renderUserList() {
        try {    
            const response = await fetch(BACKEND_URL + '/users')
        
            if (!response.ok) throw new Error('Network response was not ok');

            const data = await response.json();

            const listElement = document.getElementById('itemList')

            listElement.innerHTML = '';

            data.forEach(item => {
                const li = document.createElement('li');
                li.className = 'item';
                // Adjust 'item.name' based on what your backend JSON actually looks like
                li.textContent = item.name || "Unnamed Item"; 
                listElement.appendChild(li);
            });

        } catch (error) {
            const listElement = document.getElementById('itemList');
            console.error("Error fetching data:", error);
            listElement.innerHTML = `<li style="color:red">Failed to load data. Is the backend running?</li>`;
        }
    }

    // Home Screen Actions
    document.getElementById('actionBtn').addEventListener('click', async () => {
        const btnElement = document.getElementById('actionBtn')
        btnElement.classList.add('pressed')

        const name = JSON.parse(localStorage.getItem('pwa_user_name')).name;

        let subscriptionJson = JSON.parse(localStorage.getItem('pwa_user_name')).subObj;


        try {
        const response = await fetch(BACKEND_URL + '/send-push', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({'name': name, 'subObj': subscriptionJson})
        })

        if (!response.ok) throw new Error('Network response was not ok');

        const data = await response.json();

        } catch (error) {
            console.error("Push send failed:", error);
        }
    });

    // reset button
    document.getElementById('resetBtn').addEventListener('click', () => {
        localStorage.clear();
        onboarding.classList.remove('hidden');
        home.classList.add('hidden');
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

    return subscription
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