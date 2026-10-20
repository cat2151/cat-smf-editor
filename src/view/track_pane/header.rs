//! Track-local patch/effect controls and time-map warnings.
use crate::app::Loaded;
use cat_smf_editor::score::instrument::ResolvedLayer;
use eframe::egui::{self, Color32, RichText};

pub(super) fn open(app: &mut crate::app::App) {
    let singer = app
        .loaded
        .as_ref()
        .is_some_and(|loaded| cat_smf_editor::voicevox::is_singer(&loaded.music.tracks[app.track]));
    if singer {
        crate::view::voicevox::singer::open(app);
    } else {
        crate::view::layers::open(app);
    }
}

pub(super) fn show(loaded: &Loaded, parent: &mut egui::Ui, track: usize) -> (bool, Option<usize>) {
    let mut patch = false;
    let mut effect = None;
    // One line, so split panes start their rows at the same height.
    // Clipped to the pane: an overflowing line would widen the rows over the map beside them.
    let line = egui::Rect::from_min_size(
        parent.cursor().min,
        egui::vec2(parent.available_width(), parent.spacing().interact_size.y),
    );
    let ui = &mut parent.new_child(
        egui::UiBuilder::new()
            .max_rect(line)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    ui.set_clip_rect(line.intersect(ui.clip_rect()));
    ui.label(RichText::new(&loaded.track_titles[track]).strong());
    let layers = &loaded.patches[track].layers;
    let voice = cat_smf_editor::voicevox::Voice::read(&loaded.music.tracks[track]);
    let hover = voice.as_ref().map_or_else(
        || {
            layers
                .iter()
                .map(|l| l.label())
                .collect::<Vec<_>>()
                .join("\n")
        },
        |v| v.label(),
    );
    let names = if voice.is_some() {
        egui::text::LayoutJob::simple_singleline(
            hover.clone(),
            egui::TextStyle::Button.resolve(ui.style()),
            ui.visuals().text_color(),
        )
    } else {
        layer_names(ui, layers)
    };
    if ui
        .button(names)
        .on_hover_text(if hover.is_empty() { "—" } else { &hover })
        .clicked()
    {
        patch = true;
    }
    for (slot, plugin) in loaded.patches[track].effects.iter().enumerate() {
        let name = crate::play::mixer::effect_name(plugin);
        let color = if plugin.bypass {
            crate::play::mixer::EFFECT_COLOR.gamma_multiply(0.35)
        } else {
            crate::play::mixer::EFFECT_COLOR
        };
        if ui.button(RichText::new(name).color(color)).clicked() {
            effect = Some(slot);
        }
    }
    if let Some(map) = loaded.time.map(track) {
        let warnings: Vec<&str> = map
            .assumed_four_four
            .then_some("有効な拍子が無いため tick 0 で 4/4 とみなした。")
            .into_iter()
            .chain(map.warnings.iter().map(String::as_str))
            .collect();
        if !warnings.is_empty() {
            ui.label(RichText::new(format!("Warnings {}", warnings.len())).color(Color32::YELLOW))
                .on_hover_text(warnings.join("\n"));
        }
    }
    parent.advance_cursor_after_rect(egui::Rect::from_min_size(
        line.min,
        egui::vec2(line.width(), ui.min_rect().height()),
    ));
    (patch, effect)
}

/// Monokai accents apart from the effect blue, so neighboring layers differ.
const LAYER_COLORS: [Color32; 5] = [
    Color32::from_rgb(0xa6, 0xe2, 0x2e),
    Color32::from_rgb(0xfd, 0x97, 0x1f),
    Color32::from_rgb(0xae, 0x81, 0xff),
    Color32::from_rgb(0xf9, 0x26, 0x72),
    Color32::from_rgb(0xe6, 0xdb, 0x74),
];

/// Every layer in one button; a single layer keeps the plain text color.
fn layer_names(ui: &egui::Ui, layers: &[ResolvedLayer]) -> egui::text::LayoutJob {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let normal = ui.visuals().text_color();
    let mut job = egui::text::LayoutJob::default();
    if layers.is_empty() {
        job.append("—", 0.0, egui::TextFormat::simple(font, normal));
        return job;
    }
    for (index, layer) in layers.iter().enumerate() {
        if index > 0 {
            job.append(
                " │ ",
                0.0,
                egui::TextFormat::simple(font.clone(), normal.gamma_multiply(0.5)),
            );
        }
        let label = layer.label();
        let stem = std::path::Path::new(label)
            .file_stem()
            .map_or(label.into(), |s| s.to_string_lossy());
        let color = if layers.len() == 1 {
            normal
        } else {
            LAYER_COLORS[index % LAYER_COLORS.len()]
        };
        job.append(&stem, 0.0, egui::TextFormat::simple(font.clone(), color));
    }
    job
}
