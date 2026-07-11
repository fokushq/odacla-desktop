// =============================================================================
// Odacla Browser Extension — Background Service Worker
// =============================================================================
//
// This extension solves a critical limitation of desktop-only tracking:
// the OS can tell us "Chrome is the active window", but it can't tell us
// what URL or website the user is viewing. Only the browser itself knows that.
//
// HOW IT WORKS:
// 1. The extension listens for tab activation and URL changes
// 2. When the active tab changes, it reads the tab's URL and title
// 3. It sends this information to the Odacla desktop app
//
// COMMUNICATION WITH THE DESKTOP APP:
// We use Chrome's Native Messaging API. This creates a direct communication
// channel between the extension and a native application (our Rust/Tauri app).
//
// The flow is:
//   Browser Extension  ──native messaging──>  Fokus Desktop App
//
// For this to work, we need:
//   1. A "native messaging host" manifest file (registered in the OS)
//   2. The host application (our Tauri app) listening for messages
//
// ALTERNATIVE: If native messaging isn't set up, the extension can
// also communicate via a local HTTP endpoint (localhost).
//
// PRIVACY:
// - Only the URL and title are sent — no page content, cookies, or history
// - Incognito tabs are excluded by default
// - The data stays entirely local (Fokus doesn't phone home)
// =============================================================================

const NATIVE_HOST_NAME = "com.odacla.browser_bridge";
const POLL_INTERVAL_MS = 5000; // Match the desktop polling interval

// Port for native messaging communication with the desktop app
let nativePort = null;

// ─── Connect to the Native Messaging Host ───────────────────────────────────
// This establishes a persistent connection to the Odacla desktop app.
// If the connection fails (e.g., app not running), we retry periodically.

function connectToNativeHost() {
  try {
    nativePort = chrome.runtime.connectNative(NATIVE_HOST_NAME);

    nativePort.onMessage.addListener((message) => {
      // The desktop app can send messages back (e.g., configuration updates)
      console.log("[Odacla] Received from desktop:", message);
    });

    nativePort.onDisconnect.addListener(() => {
      console.log("[Odacla] Disconnected from desktop app. Retrying in 10s...");
      nativePort = null;
      // Retry connection after a delay
      setTimeout(connectToNativeHost, 10000);
    });

    console.log("[Odacla] Connected to desktop app via native messaging");
  } catch (error) {
    console.warn("[Odacla] Native messaging not available:", error);
    console.log("[Odacla] Falling back to localhost HTTP communication");
    nativePort = null;
  }
}

// ─── Send Activity Data to the Desktop App ──────────────────────────────────

function sendActivity(url, title) {
  const message = {
    type: "browser_activity",
    url: url,
    title: title,
    timestamp: new Date().toISOString(),
  };

  if (nativePort) {
    // Preferred: Native Messaging (direct, no network overhead)
    try {
      nativePort.postMessage(message);
    } catch (error) {
      console.warn("[Odacla] Failed to send via native messaging:", error);
    }
  } else {
    // Fallback: HTTP to localhost
    // The Tauri app can optionally listen on a local port for extension data
    fetch("http://localhost:47923/api/browser-activity", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(message),
    }).catch(() => {
      // Silently fail — the desktop app might not be running
    });
  }
}

// ─── Tab Change Listeners ───────────────────────────────────────────────────

// Fires when the user switches to a different tab
chrome.tabs.onActivated.addListener(async (activeInfo) => {
  try {
    const tab = await chrome.tabs.get(activeInfo.tabId);
    if (tab.url && !tab.incognito) {
      sendActivity(tab.url, tab.title || "");
    }
  } catch (error) {
    // Tab might have closed between the event and our query
    console.debug("[Odacla] Tab no longer available:", error);
  }
});

// Fires when the current tab's URL changes (navigation within a tab)
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  // Only send when the URL actually changes and the page is loaded
  if (changeInfo.url && tab.active && !tab.incognito) {
    sendActivity(changeInfo.url, tab.title || "");
  }
});

// Fires when the browser window gains focus
chrome.windows.onFocusChanged.addListener(async (windowId) => {
  if (windowId === chrome.windows.WINDOW_ID_NONE) return;

  try {
    const [tab] = await chrome.tabs.query({
      active: true,
      windowId: windowId,
    });
    if (tab && tab.url && !tab.incognito) {
      sendActivity(tab.url, tab.title || "");
    }
  } catch (error) {
    console.debug("[Odacla] Could not query active tab:", error);
  }
});

// ─── Periodic Polling ───────────────────────────────────────────────────────
// As a safety net, we also poll the active tab every N seconds.
// This catches edge cases where tab events don't fire (e.g., single-page apps
// that change content without changing the URL).

setInterval(async () => {
  try {
    const [tab] = await chrome.tabs.query({
      active: true,
      currentWindow: true,
    });
    if (tab && tab.url && !tab.incognito) {
      sendActivity(tab.url, tab.title || "");
    }
  } catch (error) {
    // Browser might be minimized or no active tab
  }
}, POLL_INTERVAL_MS);

// ─── Initialize ─────────────────────────────────────────────────────────────
connectToNativeHost();
console.log("[Odacla] Browser extension initialized");
