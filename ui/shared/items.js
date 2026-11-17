/* Item & pet catalog (global AP_ITEMS). The data lives in catalog.json, which the Rust core
 * also compiles in; it is handed to the UI by app_init and loaded here. */
(function (root) {
  'use strict';

  const CAT = {
    loaded: false,
    PETS: [],
    PROVIDERS: {},
    RARITY: {},
    CATEGORIES: {},
    SLOTS: [],
    ITEMS: [],
    BY_ID: {},
  };

  CAT.load = function load(c) {
    if (!c) return;
    CAT.PETS = c.pets || [];
    CAT.PROVIDERS = c.providers || {};
    CAT.RARITY = c.rarity || {};
    CAT.CATEGORIES = c.categories || {};
    CAT.SLOTS = c.slots || [];
    CAT.ITEMS = c.items || [];
    CAT.BY_ID = {};
    for (const it of CAT.ITEMS) CAT.BY_ID[it.id] = it;
    CAT.loaded = true;
  };

  CAT.pet = (id) => CAT.PETS.find((p) => p.id === id) || null;
  CAT.petForProvider = (provider) => CAT.PETS.find((p) => p.provider === provider) || null;
  CAT.item = (id) => CAT.BY_ID[id] || null;

  root.AP_ITEMS = CAT;
})(typeof self !== 'undefined' ? self : this);
