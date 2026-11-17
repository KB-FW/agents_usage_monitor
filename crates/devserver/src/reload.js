/* Injected by agentpets-devserver: live reload + a desktop-like backdrop. */
(() => {
  'use strict';
  if (window.__TAURI_INTERNALS__) return;
  const es = new EventSource('/__dev/events');
  es.onmessage = (e) => {
    if (e.data === 'css') {
      document.querySelectorAll('link[rel="stylesheet"]').forEach((l) => {
        const u = new URL(l.href);
        u.searchParams.set('v', Date.now());
        l.href = u.toString();
      });
    } else {
      location.reload();
    }
  };
  // The pet and the bar are transparent windows; outside the dashboard give them a backdrop.
  if (window.top === window) {
    const s = document.createElement('style');
    s.textContent = 'html{background:linear-gradient(135deg,#2b3a55,#4b5d7a 45%,#9aa9b8)!important;min-height:100%}';
    document.head.appendChild(s);
  }
})();
