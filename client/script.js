document.addEventListener('DOMContentLoaded', () => {
    const onboarding = document.getElementById('onboarding');
    const home = document.getElementById('home');
    const nameInput = document.getElementById('userNameInput');
    const displayName = document.getElementById('displayName');

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

        // Request Push Permission
        const permission = await Notification.requestPermission();
        
        if (permission === 'granted') {
            localStorage.setItem('pwa_user_name', name);
            showHomeScreen(name);
        } else {
            alert("Please enable notifications to continue.");
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

// Register Service Worker
if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('sw.js');
}