//! Pets & items. The single source of truth is `ui/shared/catalog.json`, compiled in here
//! and handed to the UI at start-up.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub const CATALOG_JSON: &str = include_str!("../../../ui/shared/catalog.json");

#[derive(Debug, Clone, Deserialize)]
pub struct PetDef {
    pub id: String,
    pub provider: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemDef {
    pub id: String,
    pub cat: String,
    pub rarity: u8,
    pub name: String,
    #[serde(default)]
    pub species: Option<String>,
    #[serde(default)]
    pub starter: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CategoryDef {
    pub name: String,
    #[serde(default)]
    pub slot: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Catalog {
    pub pets: Vec<PetDef>,
    pub items: Vec<ItemDef>,
    pub slots: Vec<String>,
    pub categories: BTreeMap<String, CategoryDef>,
}

impl Catalog {
    pub fn pet(&self, id: &str) -> Option<&PetDef> {
        self.pets.iter().find(|p| p.id == id)
    }
    pub fn pet_for_provider(&self, provider: &str) -> Option<&PetDef> {
        self.pets.iter().find(|p| p.provider == provider)
    }
    pub fn item(&self, id: &str) -> Option<&ItemDef> {
        self.items.iter().find(|i| i.id == id)
    }
    pub fn slot_of(&self, item: &ItemDef) -> Option<&str> {
        self.categories
            .get(&item.cat)
            .and_then(|c| c.slot.as_deref())
    }
}

pub fn catalog() -> &'static Catalog {
    static CAT: OnceLock<Catalog> = OnceLock::new();
    CAT.get_or_init(|| {
        serde_json::from_str(CATALOG_JSON).expect("ui/shared/catalog.json is invalid")
    })
}

/// The raw catalog for the UI.
pub fn catalog_value() -> serde_json::Value {
    serde_json::from_str(CATALOG_JSON).expect("ui/shared/catalog.json is invalid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_is_consistent() {
        let c = catalog();
        let ids: HashSet<_> = c.items.iter().map(|i| &i.id).collect();
        assert_eq!(ids.len(), c.items.len(), "duplicate item ids");
        for r in 1..=4 {
            assert!(
                c.items.iter().any(|i| i.rarity == r && !i.starter),
                "rarity {r} has no droppable items"
            );
        }
        for p in &c.pets {
            assert!(c.item(&format!("{}_default", p.id)).is_some());
        }
        for i in &c.items {
            assert!(
                c.categories.contains_key(&i.cat),
                "{} has unknown category",
                i.id
            );
        }
    }
}
