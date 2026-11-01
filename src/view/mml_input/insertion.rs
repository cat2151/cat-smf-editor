//! Rebuild each preview from the opening snapshot so growth is cancellable.
use super::{App, Music, State};
use cat_smf_editor::message;
use cat_smf_editor::score::pattern::{operations, Catalog};

pub(super) fn inserted(state: &State) -> Result<(Music, u64), String> {
    let mut view = state.base.clone().ok_or(message::NO_SMF_LOADED)?;
    let end = edit(state, &mut view)?;
    let mut music = if let Some(context) = &state.pattern {
        let mut saved = state
            .saved_base
            .clone()
            .ok_or("patternの編集の基準が見つかりません")?;
        let (start, until) = context.edit_range(&saved)?;
        if state.tick < start {
            return Err("MMLがpatternより前から始まっています".into());
        }
        if end > until {
            let p = Catalog::read(&saved)?.patterns[&context.pattern].clone();
            let mut meters = p.meters.clone();
            let last = meters.last().ok_or(message::MISSING_PATTERN_METER)?.clone();
            let extra = (end - until).div_ceil(last.ticks);
            if extra > 100_000u64.saturating_sub(meters.len() as u64) {
                return Err("MMLがpatternの小節数の上限を超えました".into());
            }
            meters.resize(meters.len() + extra as usize, last);
            operations::resize(&mut saved, context.pattern, meters)?;
            // Resizing can relocate storage and changes the projected meter/EOT.
            // Rebuild both sides of the mapping against that same snapshot.
            let expanded = context.project(&saved)?;
            view = expanded.clone();
            edit(state, &mut view)?;
            context.map(&saved, &expanded, &view)?
        } else {
            context.map(&saved, state.base.as_ref().unwrap(), &view)?
        }
    } else {
        view
    };
    cat_smf_editor::edit::regenerate(&mut music)?;
    Ok((music, end))
}

fn edit(state: &State, music: &mut Music) -> Result<u64, String> {
    let edit = if state.measure.is_some() {
        cat_smf_editor::edit::mml::replace
    } else {
        cat_smf_editor::edit::mml::insert
    };
    edit(music, state.track, state.tick, state.channel, &state.buffer)
}

pub(super) fn apply(app: &mut App, state: &State, music: Music) -> Result<(), String> {
    if !app.can_edit_notes(state.track) {
        return Err(message::EDIT_PATTERN_FROM_DESTINATION_TRACK.into());
    }
    if state.pattern.is_some() {
        app.apply_music(music)
    } else {
        app.apply_note_music(music, state.base.as_ref().ok_or(message::NO_SMF_LOADED)?)
    }
}
