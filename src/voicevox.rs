//! VOICEVOX humming voices, stored independently of track names.
//!
//! 譜面は `sing` / `singing_teacher` 型 style で query にし、ハミング style は query を受けて
//! 合成だけを行う。そのため query と合成で speaker が異なる。
pub mod cache;
pub mod client;
pub mod engine;
pub mod pcm;
pub mod score;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod voices_engine_tests;

/// `/sing_frame_audio_query` が受け付ける speaker（波音リツ ノーマル）。
pub const QUERY_SPEAKER: u32 = 6000;
/// `/frame_synthesis` に渡す style（ずんだもん ハミング ノーマル）。
pub const STYLE: u32 = 3003;
/// 既定のキャラクター名。歌唱の指定は専用metaで判定する。
pub const SINGER: &str = "ずんだもん";

pub mod voice;
pub use voice::{is_singer, make_singer, Voice};
