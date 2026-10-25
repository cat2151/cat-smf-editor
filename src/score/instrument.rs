//! Layer-aware runtime settings and the reference-only SMF boundary.
pub use super::layer_track::{write, Patch, ResolvedLayer};
pub fn read(
    track: &crate::model::Track,
    roots: &mut dyn crate::patch_roots::Roots,
) -> Result<Patch, String> {
    let mut patch = Patch::resolve(crate::instrument_layers::read(track, roots)?, |_| {
        Err("Patch state requires session restoration".into())
    })?;
    patch.drums = super::drums::read(track)?;
    patch.validate()?;
    Ok(patch)
}

pub use crate::instrument_layers::PREFIX;
