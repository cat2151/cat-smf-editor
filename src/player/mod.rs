//! 外部 cat-plugin-player 連携: 音色・エフェクト選択・一括選択。
#[cfg(windows)]
mod browser_activity;
#[cfg(windows)]
mod browser_loading;
pub(crate) mod bulk;
pub(crate) mod patch_roots;
#[cfg(windows)]
pub(crate) mod resident;
#[cfg(windows)]
pub(crate) mod resident_link;
pub(crate) mod selection;
pub(crate) mod state_request;
