const VAPID_PUBLIC_KEY = "BDspVj_KfBb-AOxX8zg69l74H_YRwHXr_D6mk0gdqxKy0UOqFRn1wJeD5JIvgGiSvtbq9feY0J0O4ytzaUzWxJU"; // Get this from your backend
const BACKEND_URL = "https://pager-87gw.onrender.com:443"

disablePinchZoom();

document.addEventListener('DOMContentLoaded', () => {
    const reloadSpinDurationMs = 800;
    const onboarding = document.getElementById('onboarding');
    const home = document.getElementById('home');
    const nameInput = document.getElementById('userNameInput');
    const displayName = document.getElementById('displayName');
    const getStartedBtn = document.getElementById('getStartedBtn');
    const actionBtn = document.getElementById('actionBtn');
    const resetBtn = document.getElementById('resetBtn');
    const reloadUsersBtn = document.getElementById('reloadUsersBtn');
    const serverWakeIndicator = document.getElementById('serverWakeIndicator');
    const isIOS = /iPad|iPhone|iPod/.test(navigator.userAgent) && !window.MSStream;
    const isStandalone = window.matchMedia('(display-mode: standalone)').matches || window.navigator.standalone;

    wakeBackendServer();

    // 1. Check if user already exists
    const savedName = localStorage.getItem('pwa_user_name');

    if (savedName) {
        showHomeScreen(JSON.parse(savedName).name);
    } else {
        onboarding.classList.remove('hidden');
    }

    // 2. Handle Onboarding
    bindAnimatedButton(getStartedBtn, async () => {
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
                    p256dh: arrayBufferToBase64Url(subscription.getKey('p256dh')),
                    auth: arrayBufferToBase64Url(subscription.getKey('auth'))
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
    }, { waitForActionCompletion: true });

    function showHomeScreen(name) {
        onboarding.classList.add('hidden');
        home.classList.remove('hidden');
        displayName.textContent = name;
        renderUserList()
    }

    async function renderUserList() {
        const spinStart = performance.now();

        if (reloadUsersBtn) {
            reloadUsersBtn.disabled = true;
            reloadUsersBtn.classList.add('reloading');
        }

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
                li.textContent = item || "Unnamed Item"; 
                listElement.appendChild(li);
            });

        } catch (error) {
            const listElement = document.getElementById('itemList');
            console.error("Error fetching data:", error);
            listElement.innerHTML = `<li style="color:red">Failed to load data. Is the backend running?</li>`;
        } finally {
            if (reloadUsersBtn) {
                const elapsed = performance.now() - spinStart;
                const remaining = Math.max(0, reloadSpinDurationMs - elapsed);
                if (remaining > 0) {
                    await new Promise((resolve) => window.setTimeout(resolve, remaining));
                }

                reloadUsersBtn.disabled = false;
                reloadUsersBtn.classList.remove('reloading');
            }
        }
    }

    bindAnimatedButton(reloadUsersBtn, async () => {
        await renderUserList();
    }, { waitForActionCompletion: true });

    // Home Screen Actions
    bindAnimatedButton(actionBtn, async () => {
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
    }, { waitForActionCompletion: true });

    // reset button
    bindAnimatedButton(resetBtn, () => {
        localStorage.clear();
        onboarding.classList.remove('hidden');
        home.classList.add('hidden');
    });

    function bindAnimatedButton(button, action, options = {}) {
        if (!button || typeof action !== 'function') return;

        const { waitForActionCompletion = false } = options;
        let isPointerDown = false;
        let isRunning = false;

        const runReleaseAnimation = () => {
            button.classList.remove('is-pressed', 'awaiting-completion');
            button.classList.add('is-releasing');
            window.setTimeout(() => {
                button.classList.remove('is-releasing');
            }, 280);
        };

        button.addEventListener('pointerdown', () => {
            if (button.disabled) return;
            isPointerDown = true;
            button.classList.remove('is-releasing');
            button.classList.remove('awaiting-completion');
            button.classList.add('is-pressed');
        });

        button.addEventListener('pointerup', () => {
            isPointerDown = false;
            if (isRunning && waitForActionCompletion) {
                button.classList.add('awaiting-completion');
                return;
            }
            runReleaseAnimation();
        });

        button.addEventListener('pointercancel', () => {
            isPointerDown = false;
            if (!isRunning) {
                runReleaseAnimation();
            }
        });

        button.addEventListener('click', async (event) => {
            if (button.disabled || isRunning) return;
            isRunning = true;

            if (!button.classList.contains('is-pressed')) {
                button.classList.add('is-pressed');
            }

            try {
                await action(event);
            } finally {
                isRunning = false;

                if (waitForActionCompletion) {
                    if (isPointerDown) {
                        button.classList.add('awaiting-completion');
                    } else {
                        runReleaseAnimation();
                    }
                } else if (!isPointerDown) {
                    runReleaseAnimation();
                }
            }
        });
    }

    async function wakeBackendServer() {
        setServerIndicatorState('waking', 'Waking server...');

        const timeoutController = new AbortController();
        const timeoutId = window.setTimeout(() => {
            timeoutController.abort();
        }, 40000);

        try {
            const response = await fetch(BACKEND_URL + '/status', {
                method: 'GET',
                cache: 'no-store',
                signal: timeoutController.signal
            });

            if (response.ok) {
                setServerIndicatorState('online', 'Server ready');
            } else {
                setServerIndicatorState('offline', 'Server error');
            }
        } catch (error) {
            setServerIndicatorState('offline', 'Server unreachable');
            console.error('Wake request failed:', error);
        } finally {
            window.clearTimeout(timeoutId);
        }
    }

    function setServerIndicatorState(state, text) {
        if (!serverWakeIndicator) return;

        serverWakeIndicator.classList.remove('is-waking', 'is-online', 'is-offline');

        if (state === 'online') {
            serverWakeIndicator.classList.add('is-online');
        } else if (state === 'offline') {
            serverWakeIndicator.classList.add('is-offline');
        } else {
            serverWakeIndicator.classList.add('is-waking');
        }

        const label = serverWakeIndicator.querySelector('.status-text');
        if (label) {
            label.textContent = text;
        }
    }
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

function arrayBufferToBase64Url(buffer) {
    const binary = String.fromCharCode(...new Uint8Array(buffer));
    return btoa(binary)
        .replace(/\+/g, '-')   // Replace + with -
        .replace(/\//g, '_')   // Replace / with _
        .replace(/=+$/, '');    // Remove padding =
}

// Register Service Worker
if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('sw.js');
}

function disablePinchZoom() {
    // Prevent iOS pinch gestures.
    document.addEventListener('gesturestart', (event) => {
        event.preventDefault();
    });

    // Prevent multi-touch zoom on modern mobile browsers.
    document.addEventListener('touchmove', (event) => {
        if (event.touches.length > 1) {
            event.preventDefault();
        }
    }, { passive: false });
}