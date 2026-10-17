use crate::app::App;
use eframe::egui;

fn text(app: &App) -> String {
    let mut parts = vec!["cat-smf-editor".to_owned()];
    if let Some(loaded) = &app.loaded {
        let mut filename = loaded
            .original
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if app.saved.dirty() {
            filename.push_str("(更新)");
            if app.saved.auto_save_deadline().is_some() {
                filename.push_str("(1分後にauto save)");
            }
        }
        parts.push(filename);
        if let Some(name) = loaded.music.tracks.first().and_then(|track| track.name())
            && !name.is_empty()
        {
            parts.push(name);
        }
        parts.push(format!("{}tracks", loaded.music.tracks.len()));
    }
    parts.join(" · ")
}

pub(super) fn update(app: &App, ctx: &egui::Context) {
    let title = text(app);
    let id = egui::Id::new("window-title");
    let changed = ctx.data_mut(|data| {
        if data.get_temp::<String>(id).as_ref() == Some(&title) {
            false
        } else {
            data.insert_temp(id, title.clone());
            true
        }
    });
    if changed {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cat_smf_editor::model::{Event, Music, Track};

    #[test]
    fn title_orders_filename_song_name_and_track_count_and_updates() {
        let mut app = crate::view::tests::app(Music {
            format: 1,
            division: 480,
            tracks: vec![
                Track {
                    events: vec![Event {
                        delta: 0,
                        tick: 0,
                        status: 0xff,
                        meta_type: 3,
                        data: b"Song title".to_vec(),
                    }],
                },
                Track { events: vec![] },
            ],
        });
        let loaded = app.loaded.as_mut().unwrap();
        loaded.original = "X:/music/song-2.mid".into();
        assert_eq!(
            text(&app),
            "cat-smf-editor · song-2.mid · Song title · 2tracks"
        );
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| update(&app, ctx));
        assert!(output
            .viewport_output
            .values()
            .any(|v| v.commands.iter().any(
                |c| matches!(c, egui::ViewportCommand::Title(title) if title == &text(&app))
            )));
        app.loaded = None;
        assert_eq!(text(&app), "cat-smf-editor");
    }
}
