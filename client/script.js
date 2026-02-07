document.getElementById('test-btn').addEventListener('click', async () => {
    const responseDisplay = document.getElementById('response-text');
    responseDisplay.innerText = "Connecting to Oracle Cloud...";

    try {
        // REPLACE THIS URL with your Oracle Cloud Public IP or Domain later
        const API_URL = "https://your-oracle-cloud-ip-here.com/api/test";
        
        const response = await fetch(API_URL);
        const data = await response.json();
        
        responseDisplay.innerText = `Success! Backend says: ${data.message}`;
        responseDisplay.style.color = "green";
    } catch (error) {
        responseDisplay.innerText = "Error: Could not reach backend. (Check CORS or URL)";
        responseDisplay.style.color = "red";
        console.error("Fetch error:", error);
    }
});