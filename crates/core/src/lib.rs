//! AgentMeter core: everything that is not UI.
//!
//! * [`usage`] – reads Claude Code / Codex / Gemini CLI / Grok usage from local logs and
//!   official usage endpoints, de-duplicates tokens, estimates cost, and builds snapshots
//!   with burn-rate forecasts.
//! * [`pricing`] – public API list prices used for the cost estimate.
//! * [`settings`] – persisted user settings.

pub mod pricing;
pub mod settings;
pub mod time;
pub mod usage;

pub use serde_json;
