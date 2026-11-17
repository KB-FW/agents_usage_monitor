/* Maps the UI's `window.agentPets` API onto Tauri commands & events.
 * (When previewing in a plain browser, scripts/mock-api.js provides agentPets instead.) */
(function () {
  'use strict';
  if (window.agentPets) return;
  const T = window.__TAURI__;
  if (!T) {
    console.error('AgentPets: Tauri API not available');
    return;
  }
  const invoke = T.core.invoke;
  const listen = T.event.listen;

  window.agentPets = {
    init: async () => {
      const res = await invoke('app_init');
      window.AP_ITEMS.load(res.catalog);
      return res;
    },
    action: (type, payload) => invoke('game_action', { kind: type, payload: payload || {} }),
    resetGame: () => invoke('game_reset'),
    setSettings: (patch) => invoke('settings_set', { patch }),
    refresh: () => invoke('usage_refresh'),
    openPanel: (tab) => invoke('open_panel', { tab: tab || null }),
    closeWindow: () => invoke('close_window'),
    minimizeWindow: () => invoke('minimize_window'),
    resize: (width, height) => invoke('resize_window', { width, height }),
    setHitRegions: (rects) => invoke('set_hit_regions', { rects }),
    contextMenu: () => invoke('pet_context_menu'),
    dragStart: (x, y) => invoke('drag_start', { x, y }),
    dragMove: (x, y) => invoke('drag_move', { x, y }),
    dragEnd: () => invoke('drag_end'),
    openDataDir: () => invoke('open_data_dir'),
    quit: () => invoke('quit_app'),
    on: (channel, cb) => {
      let unlisten = null;
      listen(channel, (e) => cb(e.payload)).then((fn) => (unlisten = fn));
      return () => unlisten && unlisten();
    },
  };
})();
