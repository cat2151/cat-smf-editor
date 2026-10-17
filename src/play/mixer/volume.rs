//! Volume edits update the SMF first; live gains are a projection of that value.
use crate::app::App;

pub(crate) fn set(app: &mut App, track: usize, gain: f32) -> Result<(), String> {
    change(app, track, gain, false)
}

pub(crate) fn change(app: &mut App, track: usize, gain: f32, dragging: bool) -> Result<(), String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    let source = loaded.music.tracks.get(track).ok_or("Track was removed")?;
    if !gain.is_finite()
        || !(0.0..=crate::play::track_volume::db_to_gain(crate::play::track_volume::MAX_DB))
            .contains(&gain)
    {
        return Err("Invalid track volume".into());
    }
    if crate::play::track_volume::read(source)? == gain {
        return Ok(());
    }
    let before = loaded.music.clone();
    let mut music = before.clone();
    crate::play::track_volume::write(&mut music.tracks[track], gain);
    cat_smf_editor::edit::refresh_deltas(&mut music.tracks[track])?;
    app.apply_volume_music(music)?;
    app.undo.record_volume(before, track, dragging);
    app.audition.set_gain(track, gain);
    Ok(())
}
