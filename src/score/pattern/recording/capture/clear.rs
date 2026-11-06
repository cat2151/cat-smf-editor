//! Discard captured performances while preserving containers and capture identities.
use super::*;

pub fn clear(music: &mut Music) -> Result<(), String> {
    let catalog = Catalog::read(music)?;
    let mut next = music.clone();
    for p in catalog
        .patterns
        .values()
        .filter(|p| p.kind == Kind::Capture)
    {
        // Keeping the empty definition also lets carry replace pre-clear undo snapshots.
        operations::refill(&mut next, p, vec![p.meters[0].clone()], vec![])?;
    }
    *music = next;
    Ok(())
}
