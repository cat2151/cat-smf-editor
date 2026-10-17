//! File-owned opt-in metadata and the automatic save's UI boundary.
use cat_smf_editor::{
    app_meta::AUTO_SAVE as PREFIX,
    model::{Event, Music},
};
use eframe::egui;
use std::time::{Duration, Instant};

const ON: &[u8] = b"\x01\x01"; // schema version, enabled
pub(super) const DELAY: Duration = Duration::from_secs(60);

fn owned(event: &Event) -> bool {
    event.status == 0xff && event.meta_type == 0x7f && event.data.starts_with(PREFIX)
}

pub(crate) fn enabled(music: &Music) -> bool {
    let mut records = music.tracks.iter().enumerate().flat_map(|(track, t)| {
        t.events
            .iter()
            .filter(|e| owned(e))
            .map(move |e| (track, e))
    });
    let Some((track, event)) = records.next() else {
        return false;
    };
    track == 0 && event.tick == 0 && &event.data[PREFIX.len()..] == ON && records.next().is_none()
}

fn write(music: &mut Music, enabled: bool) -> Result<(), String> {
    if music.tracks.is_empty() {
        return Err("No SMF track is loaded".into());
    }
    for track in &mut music.tracks {
        track.events.retain(|e| !owned(e));
    }
    if enabled {
        music.tracks[0].events.insert(
            0,
            Event {
                delta: 0,
                tick: 0,
                status: 0xff,
                meta_type: 0x7f,
                data: [PREFIX, ON].concat(),
            },
        );
    }
    for track in &mut music.tracks {
        cat_smf_editor::edit::refresh_deltas(track)?;
    }
    Ok(())
}

impl crate::app::App {
    pub(crate) fn toggle_auto_save(&mut self) {
        if self.config.navigation.departure.is_some() {
            return;
        }
        let Some(loaded) = &self.loaded else { return };
        if !enabled(&loaded.music) {
            self.comma_menu.open = false;
            self.allow_departure(crate::view::files::Action::EnableAutoSave);
        } else if let Err(error) = self.edit_music(|music| write(music, false)) {
            self.load_error = Some(error);
        }
    }

    pub(crate) fn enable_auto_save(&mut self) -> Result<(), String> {
        let loaded = self.loaded.as_ref().ok_or("No SMF is loaded")?;
        let before = loaded.music.clone();
        let mut music = before.clone();
        write(&mut music, true)?;
        // Keep the active document and its history untouched if persistence fails.
        let mut saved = self.saved.clone();
        saved.remember(&loaded.original, &music)?;
        saved.save()?;
        self.apply_volume_music(music)?;
        self.saved = saved;
        self.undo.push(before);
        Ok(())
    }

    pub(crate) fn auto_save(&mut self, ctx: &egui::Context) {
        self.auto_save_at(ctx, Instant::now());
    }

    pub(crate) fn auto_save_at(&mut self, ctx: &egui::Context, now: Instant) {
        let Some(deadline) = self.saved.auto_save_deadline() else {
            return;
        };
        // Let input commits, pointer gestures, confirmations and native browsers finish first.
        let busy = ctx.wants_keyboard_input()
            || ctx.input(|i| {
                i.pointer.any_down() || i.pointer.any_released() || i.viewport().close_requested()
            })
            || self.note_edit.is_some()
            || self.trill_edit.is_some()
            || !self.layer_editor.allows_undo()
            || self.config.navigation.departure.is_some()
            || self.invalid_smf_name.is_some()
            || self.file_open_requested
            || self.recent.open
            || self.player_selection.open;
        if now < deadline || busy {
            ctx.request_repaint_after(if now < deadline {
                deadline.duration_since(now)
            } else {
                Duration::from_millis(100)
            });
            return;
        }
        if let Err(error) = self.remember_loaded().and_then(|_| self.saved.save()) {
            self.saved.auto_save_failed(error);
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests;
