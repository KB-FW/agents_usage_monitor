//! Pet game rules: tokens → ✦ sparks & XP, gift bubbles, gacha, synthesis, equipment.
//! All functions mutate a [`GameState`] and return a JSON result for the UI
//! (`Err(message)` becomes `{ "error": message }`).

use crate::catalog::{catalog, ItemDef};
use crate::time::{day_key, MIN};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const TOKENS_PER_SPARK: f64 = 1000.0;
pub const TOKENS_PER_XP: f64 = 1000.0;
pub const GACHA_COST: i64 = 300;
pub const GACHA10_COST: i64 = 2700;
pub const BUBBLE_MS: i64 = 30 * MIN;
pub const MAX_GIFT_BUBBLES: usize = 2;
pub const MAX_BONUS_BUBBLES: usize = 3;
pub const BUBBLE_SPEEDUP_MS_PER_SPARK: i64 = 1500;
pub const PET_DAILY_SPARK_CAP: i64 = 100;
pub const WELCOME_SPARKS: i64 = 1000;
pub const PITY3: i64 = 10;
pub const PITY4: i64 = 60;
pub const MAX_LEVEL: u32 = 99;

/// Drop rates by rarity (★1..★4).
pub const GACHA_RATES: [f64; 4] = [0.58, 0.30, 0.10, 0.02];
pub const GIFT_RATES: [f64; 4] = [0.70, 0.24, 0.055, 0.005];
pub const BONUS_RATES: [f64; 4] = [0.45, 0.38, 0.14, 0.03];

pub type GameResult = Result<Value, String>;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Equip {
    pub skin: Option<String>,
    pub hat: Option<String>,
    pub face: Option<String>,
    pub neck: Option<String>,
    pub hand: Option<String>,
}

impl Equip {
    fn default_for(pet: &str) -> Self {
        Self {
            skin: Some(format!("{pet}_default")),
            ..Default::default()
        }
    }
    pub fn get(&self, slot: &str) -> Option<&String> {
        match slot {
            "skin" => self.skin.as_ref(),
            "hat" => self.hat.as_ref(),
            "face" => self.face.as_ref(),
            "neck" => self.neck.as_ref(),
            "hand" => self.hand.as_ref(),
            _ => None,
        }
    }
    pub fn slot_mut(&mut self, slot: &str) -> Option<&mut Option<String>> {
        match slot {
            "skin" => Some(&mut self.skin),
            "hat" => Some(&mut self.hat),
            "face" => Some(&mut self.face),
            "neck" => Some(&mut self.neck),
            "hand" => Some(&mut self.hand),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PetState {
    pub xp: f64,
    pub affection: i64,
    pub equip: Equip,
    /// Derived (refreshed before the state is sent to the UI).
    pub level: u32,
    pub progress: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bubble {
    pub id: String,
    /// gift | bonus
    pub kind: String,
    pub pet: String,
    pub ready_at: i64,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub pet: String,
    pub equip: Equip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recent {
    pub id: String,
    pub at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Petting {
    pub day: String,
    pub count: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub pulls: i64,
    pub synth: i64,
    pub gifts: i64,
    pub pets: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GameState {
    pub v: u32,
    pub installed_at: i64,
    pub sparks: i64,
    pub sparks_earned: i64,
    pub carry: BTreeMap<String, f64>,
    pub tokens: BTreeMap<String, f64>,
    pub inventory: BTreeMap<String, i64>,
    pub new_items: Vec<String>,
    pub recent: Vec<Recent>,
    pub pets: BTreeMap<String, PetState>,
    pub leader: String,
    pub presets: Vec<Option<Preset>>,
    pub bubbles: Vec<Bubble>,
    pub pity3: i64,
    pub pity4: i64,
    pub petting: Petting,
    pub stats: Stats,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Source of randomness in [0, 1).
pub trait Rng {
    fn next(&mut self) -> f64;
}

impl<F: FnMut() -> f64> Rng for F {
    fn next(&mut self) -> f64 {
        self()
    }
}

/// Small xorshift generator (seeded from the clock in the app).
pub struct XorShift(u64);

impl XorShift {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
}

impl Rng for XorShift {
    fn next(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn uid(rng: &mut dyn Rng, now: i64) -> String {
    format!(
        "{:x}{:x}",
        (rng.next() * 1e12) as u64,
        now.rem_euclid(0xfffff)
    )
}

// ------------------------------------------------------------------ levels
pub fn xp_for_level(level: u32) -> f64 {
    (1..level)
        .map(|k| (150.0 * (k as f64).powf(1.35)).round())
        .sum()
}

pub fn level_info(xp: f64) -> (u32, f64) {
    let mut level = 1;
    while level < MAX_LEVEL && xp >= xp_for_level(level + 1) {
        level += 1;
    }
    let base = xp_for_level(level);
    let next = if level >= MAX_LEVEL {
        base
    } else {
        xp_for_level(level + 1)
    };
    let progress = if next > base {
        ((xp - base) / (next - base)).clamp(0.0, 1.0)
    } else {
        1.0
    };
    (level, progress)
}

impl GameState {
    pub fn new(now: i64) -> Self {
        let cat = catalog();
        let inventory = cat
            .items
            .iter()
            .filter(|i| i.starter)
            .map(|i| (i.id.clone(), 1))
            .collect();
        let pets = cat
            .pets
            .iter()
            .map(|p| {
                (
                    p.id.clone(),
                    PetState {
                        equip: Equip::default_for(&p.id),
                        ..Default::default()
                    },
                )
            })
            .collect();
        let mut s = Self {
            v: 1,
            installed_at: now,
            sparks: WELCOME_SPARKS,
            sparks_earned: 0,
            carry: BTreeMap::new(),
            tokens: BTreeMap::new(),
            inventory,
            new_items: vec![],
            recent: vec![],
            pets,
            leader: cat.pets.first().map(|p| p.id.clone()).unwrap_or_default(),
            presets: vec![None; 5],
            bubbles: vec![],
            pity3: 0,
            pity4: 0,
            petting: Petting {
                day: day_key(now),
                count: 0,
            },
            stats: Stats::default(),
        };
        s.refresh_levels();
        s
    }

    /// Load from JSON, repairing anything missing from older saves.
    pub fn from_value(v: Option<Value>, now: i64) -> Self {
        let Some(mut s) = v
            .and_then(|v| serde_json::from_value::<GameState>(v).ok())
            .filter(|s| s.v == 1)
        else {
            return Self::new(now);
        };
        let cat = catalog();
        for p in &cat.pets {
            let ps = s.pets.entry(p.id.clone()).or_default();
            if ps.equip.skin.is_none() {
                ps.equip.skin = Some(format!("{}_default", p.id));
            }
        }
        for it in cat.items.iter().filter(|i| i.starter) {
            s.inventory.entry(it.id.clone()).or_insert(1);
        }
        if s.presets.len() != 5 {
            s.presets.resize(5, None);
        }
        if s.installed_at == 0 {
            s.installed_at = now;
        }
        if cat.pet(&s.leader).is_none() {
            s.leader = cat.pets[0].id.clone();
        }
        s.refresh_levels();
        s
    }

    pub fn refresh_levels(&mut self) {
        for p in self.pets.values_mut() {
            let (l, prog) = level_info(p.xp);
            p.level = l;
            p.progress = (prog * 1000.0).round() / 1000.0;
        }
    }

    pub fn to_value(&mut self) -> Value {
        self.refresh_levels();
        serde_json::to_value(&*self).unwrap_or(Value::Null)
    }

    // -------------------------------------------------------------- inventory
    pub fn add_item(&mut self, id: &str, now: i64) -> bool {
        let count = self.inventory.entry(id.to_string()).or_insert(0);
        let is_new = *count == 0;
        *count += 1;
        if is_new && !self.new_items.iter().any(|x| x == id) {
            self.new_items.push(id.to_string());
        }
        self.recent.retain(|r| r.id != id);
        self.recent.insert(
            0,
            Recent {
                id: id.to_string(),
                at: now,
            },
        );
        self.recent.truncate(60);
        is_new
    }

    fn remove_item(&mut self, id: &str) -> bool {
        let Some(c) = self.inventory.get_mut(id) else {
            return false;
        };
        *c -= 1;
        if *c <= 0 {
            self.inventory.remove(id);
            for p in self.pets.values_mut() {
                for slot in ["skin", "hat", "face", "neck", "hand"] {
                    if let Some(s) = p.equip.slot_mut(slot) {
                        if s.as_deref() == Some(id) {
                            *s = None;
                        }
                    }
                }
            }
        }
        true
    }

    pub fn spare_count(&self, id: &str) -> i64 {
        (self.inventory.get(id).copied().unwrap_or(0) - 1).max(0)
    }

    // -------------------------------------------------------------- tokens → sparks / xp
    /// Credit weighted tokens per provider. Returns UI events (sparks, level-ups).
    pub fn credit_tokens(&mut self, credits: &BTreeMap<String, i64>, now: i64) -> Vec<Value> {
        let cat = catalog();
        let mut events = vec![];
        let mut total = 0;
        for (provider, w) in credits {
            let w = *w as f64;
            if w <= 0.0 {
                continue;
            }
            *self.tokens.entry(provider.clone()).or_insert(0.0) += w;
            let carry = self.carry.get(provider).copied().unwrap_or(0.0) + w;
            let sparks = (carry / TOKENS_PER_SPARK).floor() as i64;
            self.carry
                .insert(provider.clone(), carry - sparks as f64 * TOKENS_PER_SPARK);
            if sparks > 0 {
                total += sparks;
                events.push(json!({ "type": "sparks", "provider": provider, "amount": sparks }));
            }
            if let Some(pet) = cat.pet_for_provider(provider) {
                let ps = self.pets.entry(pet.id.clone()).or_default();
                let before = level_info(ps.xp).0;
                ps.xp += w / TOKENS_PER_XP;
                let after = level_info(ps.xp).0;
                if after > before {
                    let bonus: i64 = (before + 1..=after).map(|l| l as i64 * 20).sum();
                    self.sparks += bonus;
                    events.push(json!({ "type": "levelup", "pet": pet.id, "level": after, "sparks": bonus }));
                }
            }
        }
        if total > 0 {
            self.sparks += total;
            self.sparks_earned += total;
            self.speed_up_bubbles(total * BUBBLE_SPEEDUP_MS_PER_SPARK, now);
        }
        self.refresh_levels();
        events
    }

    fn speed_up_bubbles(&mut self, ms: i64, now: i64) {
        if let Some(b) = self
            .bubbles
            .iter_mut()
            .filter(|b| b.ready_at > now)
            .min_by_key(|b| b.ready_at)
        {
            b.ready_at = (b.ready_at - ms).max(now);
        }
    }

    // -------------------------------------------------------------- gift bubbles
    /// Keep MAX_GIFT_BUBBLES gift timers running; returns how many were spawned.
    pub fn tick(&mut self, now: i64, rng: &mut dyn Rng) -> usize {
        let gifts = self.bubbles.iter().filter(|b| b.kind == "gift").count();
        let mut spawned = 0;
        for i in gifts..MAX_GIFT_BUBBLES {
            self.bubbles.push(Bubble {
                id: uid(rng, now + i as i64),
                kind: "gift".into(),
                pet: self.leader.clone(),
                ready_at: now + BUBBLE_MS + i as i64 * 1000,
                reason: None,
            });
            spawned += 1;
        }
        if self.petting.day != day_key(now) {
            self.petting = Petting {
                day: day_key(now),
                count: 0,
            };
        }
        spawned
    }

    /// Instant bonus gift (e.g. a usage window reset).
    pub fn add_bonus_bubble(
        &mut self,
        pet: &str,
        now: i64,
        reason: &str,
        rng: &mut dyn Rng,
    ) -> bool {
        if self.bubbles.iter().filter(|b| b.kind == "bonus").count() >= MAX_BONUS_BUBBLES {
            return false;
        }
        self.bubbles.push(Bubble {
            id: uid(rng, now),
            kind: "bonus".into(),
            pet: pet.into(),
            ready_at: now,
            reason: Some(reason.into()),
        });
        true
    }

    pub fn claim_bubble(&mut self, id: &str, now: i64, rng: &mut dyn Rng) -> GameResult {
        let idx = self
            .bubbles
            .iter()
            .position(|b| b.id == id)
            .ok_or("선물이 없어요")?;
        if self.bubbles[idx].ready_at > now {
            return Err("아직 준비 중이에요".into());
        }
        let b = self.bubbles.remove(idx);
        let rates = if b.kind == "bonus" {
            &BONUS_RATES
        } else {
            &GIFT_RATES
        };
        let item = pick_item(pick_rarity(rates, rng), rng);
        let is_new = self.add_item(&item.id, now);
        let sparks = if b.kind == "bonus" { 80 } else { 30 } + (rng.next() * 50.0) as i64;
        self.sparks += sparks;
        self.stats.gifts += 1;
        self.tick(now, rng);
        Ok(
            json!({ "item": item.id, "rarity": item.rarity, "isNew": is_new, "sparks": sparks, "pet": b.pet, "kind": b.kind }),
        )
    }

    // -------------------------------------------------------------- gacha
    pub fn gacha(&mut self, count: u32, now: i64, rng: &mut dyn Rng) -> GameResult {
        let count = if count == 10 { 10 } else { 1 };
        let cost = if count == 10 {
            GACHA10_COST
        } else {
            GACHA_COST
        };
        if self.sparks < cost {
            return Err(format!("✦가 부족해요 ({cost} 필요)"));
        }
        self.sparks -= cost;
        let mut results = vec![];
        for _ in 0..count {
            self.pity3 += 1;
            self.pity4 += 1;
            let mut rarity = pick_rarity(&GACHA_RATES, rng);
            if self.pity4 >= PITY4 {
                rarity = 4;
            } else if self.pity3 >= PITY3 && rarity < 3 {
                rarity = 3;
            }
            if rarity >= 3 {
                self.pity3 = 0;
            }
            if rarity >= 4 {
                self.pity4 = 0;
            }
            let item = pick_item(rarity, rng);
            let is_new = self.add_item(&item.id, now);
            results.push(json!({ "item": item.id, "rarity": item.rarity, "isNew": is_new }));
        }
        self.stats.pulls += count as i64;
        Ok(json!({ "results": results, "cost": cost }))
    }

    // -------------------------------------------------------------- synthesis
    pub fn synthesize(&mut self, ids: &[String], now: i64, rng: &mut dyn Rng) -> GameResult {
        let cat = catalog();
        if ids.len() != 3 {
            return Err("재료 3개를 골라주세요".into());
        }
        let items: Vec<&ItemDef> = ids
            .iter()
            .map(|id| cat.item(id).ok_or("알 수 없는 아이템"))
            .collect::<Result<_, _>>()?;
        let rarity = items[0].rarity;
        if items.iter().any(|i| i.rarity != rarity) {
            return Err("같은 등급끼리만 합성할 수 있어요".into());
        }
        if rarity >= 4 {
            return Err("전설 등급은 합성할 수 없어요".into());
        }
        let mut need: BTreeMap<&str, i64> = BTreeMap::new();
        for id in ids {
            *need.entry(id.as_str()).or_insert(0) += 1;
        }
        if need.iter().any(|(id, n)| self.spare_count(id) < *n) {
            return Err("중복 아이템이 부족해요 (한 개는 남겨둬요)".into());
        }
        for id in ids {
            self.remove_item(id);
        }
        let target = if rarity == 1 && rng.next() < 0.05 {
            3
        } else {
            rarity + 1
        };
        let item = pick_item(target, rng);
        let is_new = self.add_item(&item.id, now);
        self.stats.synth += 1;
        Ok(json!({ "item": item.id, "rarity": item.rarity, "isNew": is_new, "consumed": ids }))
    }

    /// Suggest 3 spare copies of the given rarity (most duplicated first).
    pub fn auto_pick_synth(&self, rarity: u8) -> Option<Vec<String>> {
        let mut spares: Vec<&ItemDef> = catalog()
            .items
            .iter()
            .filter(|i| i.rarity == rarity && self.spare_count(&i.id) > 0)
            .collect();
        spares.sort_by_key(|i| -self.spare_count(&i.id));
        let mut picks = vec![];
        for it in spares {
            for _ in 0..self.spare_count(&it.id) {
                if picks.len() < 3 {
                    picks.push(it.id.clone());
                }
            }
        }
        (picks.len() == 3).then_some(picks)
    }

    // -------------------------------------------------------------- equipment & presets
    pub fn equip(&mut self, pet: &str, item_id: &str) -> GameResult {
        let cat = catalog();
        let it = cat.item(item_id).ok_or("알 수 없는 아이템")?;
        if !self.pets.contains_key(pet) {
            return Err("알 수 없는 펫".into());
        }
        if self.inventory.get(item_id).copied().unwrap_or(0) <= 0 {
            return Err("가지고 있지 않은 아이템이에요".into());
        }
        let slot = cat
            .slot_of(it)
            .ok_or("장착할 수 없는 아이템이에요")?
            .to_string();
        let ps = self.pets.get_mut(pet).unwrap();
        if slot == "skin" {
            if it.species.as_deref() != Some(pet) {
                return Err("이 펫의 스킨이 아니에요".into());
            }
            ps.equip.skin = Some(item_id.into());
            return Ok(json!({ "slot": slot, "equipped": true }));
        }
        let s = ps.equip.slot_mut(&slot).unwrap();
        if s.as_deref() == Some(item_id) {
            *s = None;
            return Ok(json!({ "slot": slot, "equipped": false }));
        }
        *s = Some(item_id.into());
        Ok(json!({ "slot": slot, "equipped": true }))
    }

    pub fn unequip(&mut self, pet: &str, slot: &str) -> GameResult {
        let ps = self.pets.get_mut(pet).ok_or("알 수 없는 펫")?;
        let s = ps.equip.slot_mut(slot).ok_or("잘못된 슬롯")?;
        *s = if slot == "skin" {
            Some(format!("{pet}_default"))
        } else {
            None
        };
        Ok(json!({ "slot": slot }))
    }

    pub fn save_preset(&mut self, idx: usize, pet: &str) -> GameResult {
        let ps = self.pets.get(pet).ok_or("잘못된 프리셋")?;
        if idx >= 5 {
            return Err("잘못된 프리셋".into());
        }
        self.presets[idx] = Some(Preset {
            pet: pet.into(),
            equip: ps.equip.clone(),
        });
        Ok(json!({ "idx": idx }))
    }

    pub fn apply_preset(&mut self, idx: usize) -> GameResult {
        let p = self
            .presets
            .get(idx)
            .cloned()
            .flatten()
            .ok_or("빈 프리셋이에요")?;
        if !self.pets.contains_key(&p.pet) {
            return Err("빈 프리셋이에요".into());
        }
        let mut eq = Equip::default();
        for slot in ["skin", "hat", "face", "neck", "hand"] {
            let owned = p
                .equip
                .get(slot)
                .filter(|id| self.inventory.get(*id).copied().unwrap_or(0) > 0)
                .cloned();
            *eq.slot_mut(slot).unwrap() = if slot == "skin" {
                owned.or(Some(format!("{}_default", p.pet)))
            } else {
                owned
            };
        }
        self.pets.get_mut(&p.pet).unwrap().equip = eq;
        self.leader = p.pet.clone();
        Ok(json!({ "pet": p.pet }))
    }

    pub fn clear_preset(&mut self, idx: usize) -> GameResult {
        if idx >= 5 {
            return Err("잘못된 프리셋".into());
        }
        self.presets[idx] = None;
        Ok(json!({ "idx": idx }))
    }

    pub fn set_leader(&mut self, pet: &str) -> GameResult {
        if !self.pets.contains_key(pet) {
            return Err("알 수 없는 펫".into());
        }
        self.leader = pet.into();
        for b in self.bubbles.iter_mut().filter(|b| b.kind == "gift") {
            b.pet = pet.into();
        }
        Ok(json!({ "leader": pet }))
    }

    pub fn pet_pet(&mut self, pet: &str, now: i64) -> GameResult {
        if !self.pets.contains_key(pet) {
            return Err("알 수 없는 펫".into());
        }
        if self.petting.day != day_key(now) {
            self.petting = Petting {
                day: day_key(now),
                count: 0,
            };
        }
        let ps = self.pets.get_mut(pet).unwrap();
        ps.affection += 1;
        let affection = ps.affection;
        self.stats.pets += 1;
        let mut sparks = 0;
        if self.petting.count < PET_DAILY_SPARK_CAP {
            sparks = 1;
            self.petting.count += 1;
            self.sparks += 1;
        }
        Ok(json!({ "affection": affection, "sparks": sparks }))
    }

    pub fn mark_seen(&mut self, ids: Option<&[String]>) -> GameResult {
        match ids {
            None => self.new_items.clear(),
            Some(ids) => self.new_items.retain(|x| !ids.contains(x)),
        }
        Ok(json!({}))
    }

    /// Dispatch a UI action. Returns `(result, changed_state)`.
    pub fn action(
        &mut self,
        kind: &str,
        p: &Value,
        now: i64,
        rng: &mut dyn Rng,
    ) -> (GameResult, bool) {
        let s = |k: &str| p.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let idx = p.get("idx").and_then(|v| v.as_u64()).unwrap_or(99) as usize;
        let res = match kind {
            "gacha" => self.gacha(
                p.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
                now,
                rng,
            ),
            "claimBubble" => self.claim_bubble(&s("id"), now, rng),
            "synth" => {
                let ids: Vec<String> = p
                    .get("ids")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                self.synthesize(&ids, now, rng)
            }
            "autoSynth" => {
                let r = p.get("rarity").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                return (Ok(json!({ "ids": self.auto_pick_synth(r) })), false);
            }
            "equip" => self.equip(&s("pet"), &s("item")),
            "unequip" => self.unequip(&s("pet"), &s("slot")),
            "savePreset" => self.save_preset(idx, &s("pet")),
            "applyPreset" => self.apply_preset(idx),
            "clearPreset" => self.clear_preset(idx),
            "setLeader" => self.set_leader(&s("pet")),
            "pet" => self.pet_pet(&s("pet"), now),
            "markSeen" => {
                let ids: Option<Vec<String>> = p
                    .get("ids")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                self.mark_seen(ids.as_deref())
            }
            "emote" => return (Ok(json!({})), false),
            other => Err(format!("unknown action {other}")),
        };
        let changed = res.is_ok();
        (res, changed)
    }
}

fn pick_rarity(rates: &[f64; 4], rng: &mut dyn Rng) -> u8 {
    let mut r = rng.next();
    for k in (1..=4u8).rev() {
        let p = rates[(k - 1) as usize];
        if r < p {
            return k;
        }
        r -= p;
    }
    1
}

fn pick_item(rarity: u8, rng: &mut dyn Rng) -> &'static ItemDef {
    let cat = catalog();
    let mut r = rarity;
    loop {
        let pool: Vec<&ItemDef> = cat
            .items
            .iter()
            .filter(|i| i.rarity == r && !i.starter)
            .collect();
        if !pool.is_empty() {
            let i = ((rng.next() * pool.len() as f64) as usize).min(pool.len() - 1);
            return pool[i];
        }
        if r <= 1 {
            return cat
                .items
                .iter()
                .find(|i| !i.starter)
                .expect("catalog has droppable items");
        }
        r -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed(v: f64) -> impl FnMut() -> f64 {
        move || v
    }

    #[test]
    fn new_state_has_starters() {
        let s = GameState::new(0);
        assert_eq!(s.sparks, WELCOME_SPARKS);
        for p in &catalog().pets {
            assert_eq!(s.inventory.get(&format!("{}_default", p.id)), Some(&1));
            assert_eq!(
                s.pets[&p.id].equip.skin.as_deref(),
                Some(format!("{}_default", p.id).as_str())
            );
        }
        assert!(s.inventory.contains_key("emo_heart"));
    }

    #[test]
    fn repairs_partial_saves() {
        let s = GameState::from_value(
            Some(
                json!({ "v": 1, "sparks": 5, "pets": { "sunny": { "xp": 10, "equip": { "skin": "sunny_mint" } } } }),
            ),
            0,
        );
        assert_eq!(s.sparks, 5);
        assert_eq!(s.pets["sunny"].equip.hat, None);
        assert_eq!(s.pets["sunny"].equip.skin.as_deref(), Some("sunny_mint"));
        assert!(s.pets.contains_key("orbit"));
        assert!(s.inventory.contains_key("orbit_default"));
        assert_eq!(s.presets.len(), 5);
        assert_eq!(GameState::from_value(None, 7).installed_at, 7);
        // the JSON contract the UI reads
        let mut s = s;
        let v = s.to_value();
        assert!(v["pets"]["sunny"].get("level").is_some());
        assert!(v.get("newItems").is_some() && v.get("installedAt").is_some());
    }

    #[test]
    fn tokens_become_sparks_and_levels() {
        let mut s = GameState::new(0);
        let start = s.sparks;
        let c = |n: i64| BTreeMap::from([("claude".to_string(), n)]);
        s.credit_tokens(&c(1500), 0);
        assert_eq!(s.sparks, start + 1);
        assert_eq!(s.carry["claude"], 500.0);
        s.credit_tokens(&c(600), 0);
        assert_eq!(s.sparks, start + 2);
        let ev = s.credit_tokens(&c(2_000_000), 0);
        let lv = ev
            .iter()
            .find(|e| e["type"] == "levelup")
            .expect("level up");
        assert_eq!(lv["pet"], "sunny");
        assert_eq!(lv["level"].as_u64().unwrap() as u32, s.pets["sunny"].level);
        let mut prev = 0;
        for xp in (0..500_000).step_by(997) {
            let l = level_info(xp as f64).0;
            assert!(l >= prev);
            prev = l;
        }
    }

    #[test]
    fn bubbles_spawn_speed_up_and_claim() {
        let mut s = GameState::new(0);
        let mut rng = XorShift::new(42);
        s.tick(0, &mut rng);
        assert_eq!(s.bubbles.len(), MAX_GIFT_BUBBLES);
        let first = s.bubbles[0].id.clone();
        assert_eq!(
            s.claim_bubble(&first, 1000, &mut rng).unwrap_err(),
            "아직 준비 중이에요"
        );
        s.credit_tokens(&BTreeMap::from([("codex".into(), 10_000)]), 1000);
        assert_eq!(s.bubbles[0].ready_at, BUBBLE_MS - 15_000);
        let r = s.claim_bubble(&first, BUBBLE_MS, &mut rng).unwrap();
        assert!(s.inventory.contains_key(r["item"].as_str().unwrap()));
        assert_eq!(
            s.bubbles.iter().filter(|b| b.kind == "gift").count(),
            MAX_GIFT_BUBBLES
        );

        for _ in 0..5 {
            s.add_bonus_bubble("sunny", 0, "reset", &mut rng);
        }
        assert_eq!(
            s.bubbles.iter().filter(|b| b.kind == "bonus").count(),
            MAX_BONUS_BUBBLES
        );
    }

    #[test]
    fn gacha_costs_and_pity() {
        let mut s = GameState::new(0);
        s.sparks = 100;
        assert!(s.gacha(1, 0, &mut fixed(0.5)).is_err());
        s.sparks = 1_000_000;
        let mut worst = fixed(0.999);
        for _ in 0..9 {
            s.gacha(1, 0, &mut worst).unwrap();
        }
        let tenth = s.gacha(1, 0, &mut worst).unwrap();
        assert_eq!(tenth["results"][0]["rarity"], 3, "pity ★3 on the 10th pull");
        let mut got4 = false;
        for _ in 0..6 {
            let r = s.gacha(10, 0, &mut worst).unwrap();
            got4 |= r["results"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x["rarity"] == 4);
        }
        assert!(got4, "pity ★4");
        for it in catalog().items.iter().filter(|i| i.starter) {
            assert_eq!(
                s.inventory.get(&it.id).copied().unwrap_or(1),
                1,
                "{} must not drop",
                it.id
            );
        }
    }

    #[test]
    fn synthesis_uses_spares_only() {
        let mut s = GameState::new(0);
        s.inventory.insert("hat_cap".into(), 2);
        s.inventory.insert("hat_bow".into(), 3);
        let ids = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert!(s
            .synthesize(&ids(&["hat_cap", "hat_cap", "hat_bow"]), 0, &mut fixed(0.5))
            .is_err());
        let pick = s.auto_pick_synth(1).unwrap();
        let r = s.synthesize(&pick, 0, &mut fixed(0.5)).unwrap();
        assert_eq!(r["rarity"], 2);
        assert!(s.inventory["hat_cap"] >= 1 && s.inventory["hat_bow"] >= 1);
        assert!(s
            .synthesize(
                &ids(&["hat_crown", "hat_crown", "hat_crown"]),
                0,
                &mut fixed(0.5)
            )
            .is_err());
    }

    #[test]
    fn equip_presets_leader_and_petting() {
        let mut s = GameState::new(0);
        assert!(s.equip("sunny", "hat_crown").is_err());
        s.inventory.insert("hat_crown".into(), 1);
        s.inventory.insert("nimbus_night".into(), 1);
        assert_eq!(s.equip("sunny", "hat_crown").unwrap()["equipped"], true);
        assert!(s.equip("sunny", "nimbus_night").is_err());
        s.save_preset(0, "sunny").unwrap();
        assert_eq!(s.equip("sunny", "hat_crown").unwrap()["equipped"], false);
        s.set_leader("orbit").unwrap();
        s.apply_preset(0).unwrap();
        assert_eq!(s.leader, "sunny");
        assert_eq!(s.pets["sunny"].equip.hat.as_deref(), Some("hat_crown"));
        assert!(s.apply_preset(3).is_err());

        let start = s.sparks;
        for _ in 0..PET_DAILY_SPARK_CAP + 20 {
            s.pet_pet("twinkle", 0).unwrap();
        }
        assert_eq!(s.sparks - start, PET_DAILY_SPARK_CAP);
        s.pet_pet("twinkle", 2 * 24 * 3600 * 1000).unwrap();
        assert_eq!(s.sparks - start, PET_DAILY_SPARK_CAP + 1);
    }

    #[test]
    fn rates_sum_to_one() {
        for r in [GACHA_RATES, GIFT_RATES, BONUS_RATES] {
            assert!((r.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        }
        let mut x = XorShift::new(7);
        for _ in 0..1000 {
            let v = x.next();
            assert!((0.0..1.0).contains(&v));
        }
    }
}
