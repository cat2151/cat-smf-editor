pub mod app_meta;
pub mod edit;
#[path = "../../cat-plugin-player/shared/instrument_layers.rs"]
pub mod instrument_layers;
#[path = "../../cat-plugin-player/shared/master_references.rs"]
pub mod master_references;
#[path = "../../cat-plugin-player/shared/patch_reference.rs"]
pub mod patch_reference;
#[path = "../../cat-plugin-player/shared/patch_roots.rs"]
pub mod patch_roots;
pub mod score;
pub mod storage_paths;

#[path = "../../cat-plugin-player/shared/smf.rs"]
pub mod smf;
pub use smf::{ffi, io as smf_io, model};
#[path = "../../cat-plugin-player/shared/chain_settings.rs"]
pub mod chain_settings;
#[path = "../../cat-plugin-player/shared/ffi.rs"]
pub mod host;
#[path = "../../cat-plugin-player/shared/master_effects.rs"]
pub mod master_effects;

#[path = "../../cat-plugin-player/shared/chain.rs"]
pub mod chain;
#[path = "../../cat-plugin-player/shared/native_library.rs"]
pub mod native_library;
#[path = "../../cat-plugin-player/shared/note_render.rs"]
pub mod note_render;

#[path = "../../cat-plugin-player/shared/smf_timeline.rs"]
pub mod smf_timeline;

#[cfg(windows)]
#[path = "../../cat-plugin-player/shared/resident_ipc.rs"]
pub mod resident_ipc;
#[path = "../../cat-plugin-player/shared/resident_log.rs"]
pub mod resident_log;

#[path = "../../cat-plugin-player/shared/patch_identity.rs"]
pub mod patch_identity;

#[path = "../../cat-plugin-player/shared/correlation.rs"]
pub mod correlation;
#[path = "../../cat-plugin-player/shared/correlation_ui.rs"]
pub mod correlation_ui;
#[path = "../../cat-plugin-player/shared/lissajous.rs"]
pub mod lissajous;
#[path = "../../cat-plugin-player/shared/lissajous_ui.rs"]
pub mod lissajous_ui;
#[path = "../../cat-plugin-player/shared/spectrum.rs"]
pub mod spectrum;
#[path = "../../cat-plugin-player/shared/spectrum_ui.rs"]
pub mod spectrum_ui;
