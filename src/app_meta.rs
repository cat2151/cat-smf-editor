//! Prefixes of the editor's own sequencer-specific metas whose owners live in the app crate,
//! here so the event list can decode them.
pub const TRACK_VOLUME: &[u8] = b"\x7dcat-smf-editor\0volume\0";
pub const AUTO_SAVE: &[u8] = b"\x7dcat-smf-editor\0auto-save\0";
