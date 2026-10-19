//! MID・session・設定・config・error log・起動引数の永続化と読み込み。
pub(crate) mod auto_save;
mod backup;
pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod error_log;
pub(crate) mod navigation;
pub(crate) mod saved;
mod saved_write;
pub(crate) mod session;
pub(crate) mod settings;
pub(crate) mod startup;

pub(crate) mod recent;
