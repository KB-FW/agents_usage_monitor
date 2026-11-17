//! AgentPets core: everything that is not UI.
//!
//! * [`usage`] – reads Claude Code / Codex / Gemini CLI / Grok usage from local logs and
//!   official usage endpoints, de-duplicates tokens and builds snapshots.
//! * [`game`] – the pet game rules (sparks, gift bubbles, gacha, synthesis, presets).
//! * [`settings`] – persisted user settings.
//! * [`catalog`] – pets & items (shared with the UI through `ui/shared/catalog.json`).

pub mod catalog;
pub mod game;
pub mod settings;
pub mod time;
pub mod usage;

pub use serde_json;
