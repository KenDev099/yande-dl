//! JSON config persistence for yande-dl.
//!
//! - [`paths::AppPaths`] resolves cross-platform config paths.
//! - [`atomic_write`] handles tmp + rename writes.
//! - [`tags::TagsStore`] manages subscriptions in `tags.json`.
//! - [`settings::SettingsStore`] manages user settings in `settings.json`.

pub mod atomic_write;
pub mod paths;
pub mod settings;
pub mod tags;

pub use paths::AppPaths;
pub use settings::{Settings, SettingsStore};
pub use tags::{ImportMode, ImportReport, Subscription, TagsFile, TagsStore};
