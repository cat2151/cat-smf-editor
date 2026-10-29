//! Annotations shared by both tick-based overlays; managed trills open an editor.
use crate::{app::Loaded, store::config::Store};
use eframe::egui::{self, Color32};

const COLOR: Color32 = Color32::from_rgb(0xe6, 0xdb, 0x74);

pub(super) fn guide_track(loaded: &Loaded, tracks: &[usize]) -> Option<usize> {
    if loaded.display_music.is_some()
        || tracks
            .iter()
            .any(|&t| cat_smf_editor::score::pattern::is_source(&loaded.music, t))
    {
        return None;
    }
    cat_smf_editor::score::chord_guide::track(&loaded.music)
}

/// A silent guide still contributes its annotation positions to the viewport extent.
pub(super) fn guide_end(loaded: &Loaded, tracks: &[usize]) -> u64 {
    guide_track(loaded, tracks)
        .and_then(|t| loaded.music.tracks[t].events.last())
        .map_or(0, |e| e.tick)
}

pub(super) fn show(
    ui: &mut egui::Ui,
    loaded: &Loaded,
    track: usize,
    config: &Store,
    rect: egui::Rect,
    scale: f32,
) -> Option<(usize, usize)> {
    paint(
        ui,
        &loaded.music,
        config,
        rect,
        scale,
        guide_track(loaded, &[track]).is_some(),
    )
}

pub(super) fn show_music(
    ui: &mut egui::Ui,
    music: &cat_smf_editor::model::Music,
    config: &Store,
    rect: egui::Rect,
    scale: f32,
) -> Option<(usize, usize)> {
    paint(ui, music, config, rect, scale, true)
}

fn paint(
    ui: &mut egui::Ui,
    music: &cat_smf_editor::model::Music,
    config: &Store,
    rect: egui::Rect,
    scale: f32,
    include_guide: bool,
) -> Option<(usize, usize)> {
    let guide = include_guide
        .then(|| cat_smf_editor::score::chord_guide::track(music))
        .flatten();
    let clip = rect.intersect(ui.clip_rect());
    let painter = ui.painter().with_clip_rect(clip);
    let mut events: Vec<_> = music
        .tracks
        .iter()
        .enumerate()
        .flat_map(|(track, t)| {
            t.events.iter().enumerate().filter_map(move |(index, e)| {
                ((config.meta_text_visible
                    && e.status == 0xff
                    && e.meta_type == 0x01
                    && !e.data.is_empty())
                    || (guide == Some(track)
                        && cat_smf_editor::score::chord_guide::chord(e).is_some()))
                .then_some((e.tick, track, index, e))
            })
        })
        .collect();
    events.sort_by_key(|&(tick, track, index, _)| (tick, track, index));
    // Each lane stores the right edge and height of its last label.
    let mut lanes: Vec<(f32, f32)> = Vec::new();
    let mut clicked = None;
    for (tick, track, index, event) in events {
        let text = cat_smf_editor::score::chord_guide::chord(event).map_or_else(
            || String::from_utf8_lossy(&event.data),
            std::borrow::Cow::Borrowed,
        );
        if text.trim().is_empty() {
            continue;
        }
        let galley = painter.layout_no_wrap(
            cat_smf_editor::edit::trill::label(event)
                .map_or_else(|| text.replace(['\r', '\n', '\t'], " "), str::to_owned),
            egui::FontId::proportional(13.0),
            COLOR,
        );
        let size = galley.size() + egui::vec2(6.0, 4.0);
        let x = rect.left() + tick as f32 * scale;
        if x > clip.right() || x + size.x < clip.left() {
            continue;
        }
        let lane = lanes.iter().position(|&(right, _)| right + 3.0 <= x);
        let lane = lane.unwrap_or_else(|| {
            lanes.push((f32::NEG_INFINITY, 0.0));
            lanes.len() - 1
        });
        let y = clip.top() + 2.0 + lanes[..lane].iter().map(|l| l.1 + 2.0).sum::<f32>();
        lanes[lane] = (x + size.x, lanes[lane].1.max(size.y));
        let label = egui::Rect::from_min_size(egui::pos2(x, y), size);
        if !label.intersects(clip) {
            continue;
        }
        painter.rect_filled(label, 2.0, Color32::from_rgb(30, 30, 30));
        painter.galley(label.min + egui::vec2(3.0, 2.0), galley, COLOR);
        let editable = cat_smf_editor::edit::trill::owned(event);
        let response = ui
            .interact(
                label.intersect(clip),
                ui.id()
                    .with(("meta-text", track, index, rect.top().to_bits())),
                if editable {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                },
            )
            .on_hover_text(text.as_ref());
        // The tempo canvas also senses clicks across its entire rectangle. A
        // label owns a primary click inside its visible bounds even if that
        // enclosing canvas acquired the pointer first.
        let label_clicked = ui.rect_contains_pointer(label.intersect(clip))
            && ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary));
        if editable && (response.clicked() || label_clicked) {
            clicked = Some((track, index));
        }
    }
    clicked
}

#[cfg(test)]
mod tests;
