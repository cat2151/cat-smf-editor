//! MID・session・設定・config・error log・起動引数の永続化と読み込み。
pub(crate) mod auto_save;
mod backup;
pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod error_log;
pub(crate) mod navigation;
pub(crate) mod saved;
mod saved_write;
pub(crate) use saved_write::replace_with_backup;
pub(crate) mod session;
pub(crate) mod settings;
pub(crate) mod startup;

#[cfg(test)]
mod pattern_persistence_tests;
pub(crate) mod recent;

#[cfg(test)]
mod scene_persistence_tests;
