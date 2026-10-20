use cat_smf_editor::{model::Event, score::musical_time::Position};
use eframe::egui::{self, Color32, RichText, ScrollArea};
const MONOKAI_YELLOW: Color32 = Color32::from_rgb(0xe6, 0xdb, 0x74);
pub(crate) fn beat_color(beat: u128) -> Color32 {
    match (beat - 1) % 4 {
        0 => Color32::from_rgb(0x66, 0xd9, 0xef),
        1 => Color32::from_rgb(0xf9, 0x26, 0x72),
        2 => Color32::from_rgb(0xfd, 0x97, 0x1f),
        _ => Color32::from_rgb(0xa6, 0xe2, 0x2e),
    }
}

/// Scales RGB only; gamma_multiply would also lower alpha.
fn dim(color: Color32) -> Color32 {
    let scale = |c: u8| (f32::from(c) * 0.55) as u8;
    Color32::from_rgb(scale(color.r()), scale(color.g()), scale(color.b()))
}

/// Only the beat number is colored, and the selection color does not apply.
fn mbt_job(ui: &egui::Ui, mbt: &str, beat: Color32) -> egui::text::LayoutJob {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let normal = ui.visuals().text_color();
    let start = mbt.find(':').map_or(mbt.len(), |i| i + 1);
    let end = mbt[start..].find(':').map_or(mbt.len(), |i| start + i);
    let mut job = egui::text::LayoutJob::default();
    for (part, color) in [
        (&mbt[..start], normal),
        (&mbt[start..end], beat),
        (&mbt[end..], normal),
    ] {
        job.append(part, 0.0, egui::TextFormat::simple(font.clone(), color));
    }
    job
}

/// Text with its own color, or `None` for the row's text color.
pub(crate) type Span = (String, Option<Color32>);

fn spans_job(
    ui: &egui::Ui,
    spans: &[Span],
    plain: Color32,
    monospace: bool,
) -> egui::text::LayoutJob {
    let font = if monospace {
        egui::TextStyle::Monospace
    } else {
        egui::TextStyle::Body
    }
    .resolve(ui.style());
    let mut job = egui::text::LayoutJob::default();
    for (text, color) in spans {
        job.append(
            text,
            0.0,
            egui::TextFormat::simple(font.clone(), color.unwrap_or(plain)),
        );
    }
    job
}

pub(crate) struct RowStyle<'a> {
    pub(crate) note_mode: bool,
    pub(crate) selected: bool,
    pub(crate) playing: bool,
    pub(crate) preparing: bool,
    pub(crate) boundary: Option<egui::Stroke>,
    /// Shares its tick with the first row of its beat.
    pub(crate) beat_head: bool,
    pub(crate) position: Option<&'a Position>,
    pub(crate) raw_event: Option<&'a Event>,
    pub(crate) pitches: &'a [(u8, bool)],
    pub(crate) key: Option<cat_smf_editor::score::key_signature::KeySignature>,
    pub(crate) pitch_range: Option<(u8, u8)>,
    /// Note mode: MBT at index 3. Raw: every column; a shorter row's last cell takes the rest.
    pub(crate) widths: &'a [f32],
    /// Column drawn in Monokai yellow.
    pub(crate) caution: Option<usize>,
    /// A column drawn as colored pieces; `None` pieces take the row's text color.
    pub(crate) spans: Option<(usize, Vec<Span>)>,
}

pub(crate) struct RowResponse {
    /// The clicked column; `cells.len()` past the last column.
    pub(crate) clicked: Option<usize>,
    pub(crate) cells: Vec<egui::Rect>,
}

pub(crate) fn row(ui: &mut egui::Ui, cells: &[&str], style: RowStyle<'_>) -> RowResponse {
    let RowStyle {
        note_mode,
        selected,
        playing,
        preparing,
        boundary,
        beat_head,
        position,
        raw_event,
        pitches,
        key,
        pitch_range,
        widths: column_widths,
        caution,
        spans,
    } = style;
    let row_width = ui.available_width();
    let widths = if note_mode && cells.len() == 2 {
        vec![
            column_widths[3],
            (row_width - column_widths[3] - 4.0).max(726.0),
        ]
    } else if note_mode {
        vec![column_widths[3], 90.0, 70.0, 50.0, 500.0]
    } else {
        let mut widths = column_widths.to_vec();
        if cells.len() < widths.len() {
            widths.truncate(cells.len());
            let used: f32 = widths[..cells.len() - 1].iter().sum();
            widths[cells.len() - 1] = (row_width - used).max(200.0);
        }
        widths
    };
    let mut clicked = None;
    let mut rects = Vec::with_capacity(cells.len());
    let height = ui.text_style_height(&egui::TextStyle::Body) + 2.0;
    if !pitches.is_empty() {
        // Painted before the cells so their text stays on top.
        crate::view::pitch_keyboard::backdrop(
            ui,
            egui::Rect::from_min_size(ui.cursor().min, egui::vec2(500.0, height)),
            pitches,
            pitch_range.unwrap_or((pitches[0].0, pitches[0].0)),
            key,
            playing,
        );
    }
    // The row senses clicks under its cells, so the space after the last column selects too.
    let response = ui.scope_builder(
        egui::UiBuilder::new()
            // The same starting rect as ui.horizontal, so rows keep their height and spacing.
            .max_rect(egui::Rect::from_min_size(
                ui.cursor().min,
                egui::vec2(row_width, ui.spacing().interact_size.y),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center))
            .sense(egui::Sense::click()),
        |ui| {
            ui.set_min_width(row_width);
            ui.spacing_mut().item_spacing.x = if note_mode { 4.0 } else { raw_spacing(ui) };
            for (column, (cell, width)) in cells.iter().zip(widths).enumerate() {
                let mut text = if caution == Some(column) {
                    RichText::new(*cell).color(MONOKAI_YELLOW)
                } else if selected {
                    RichText::new(*cell).color(Color32::LIGHT_BLUE)
                } else {
                    RichText::new(*cell)
                };
                let mbt_column = column == if note_mode { 0 } else { 3 };
                if !note_mode || mbt_column {
                    text = text.monospace();
                }
                let text: egui::WidgetText = match position {
                    Some(position) if mbt_column => {
                        // A boundary is drawn exactly on the first row of each beat.
                        let color = beat_color(position.beat);
                        let beat = if boundary.is_some() {
                            color
                        } else if beat_head {
                            dim(color)
                        } else {
                            ui.visuals().text_color()
                        };
                        mbt_job(ui, cell, beat).into()
                    }
                    _ => match &spans {
                        Some((at, spans)) if *at == column => {
                            let plain = if selected {
                                Color32::LIGHT_BLUE
                            } else {
                                ui.visuals().text_color()
                            };
                            spans_job(ui, spans, plain, !note_mode).into()
                        }
                        _ => text.into(),
                    },
                };
                // Reserve the entire column; allocate_ui_with_layout shrinks to its contents.
                let (rect, mut cell_response) =
                    ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
                rects.push(rect);
                let mut cell_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                cell_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
                // Text selection would capture the click before the owning row sees it.
                cell_ui.add(
                    egui::Label::new(text)
                        .selectable(false)
                        .halign(egui::Align::LEFT)
                        .truncate(),
                );
                if column == RAW_COLUMN
                    && let Some(event) = raw_event
                {
                    cell_response = cell_response.on_hover_ui(|ui| {
                        ui.label(format!("{} bytes", event.data.len()));
                        if ui.button("Copy").clicked() {
                            ui.ctx().copy_text(raw_full(event));
                        }
                        ScrollArea::both().max_height(300.0).show(ui, |ui| {
                            ui.monospace(raw_full(event));
                        });
                    });
                }
                if cell_response.clicked() {
                    clicked = Some(column);
                }
            }
        },
    );
    if clicked.is_none() && response.response.clicked() {
        clicked = Some(cells.len());
    }
    if let Some(stroke) = boundary {
        let rect = response.response.rect;
        ui.painter().hline(rect.x_range(), rect.top(), stroke);
    }
    if playing {
        let rect = response.response.rect;
        ui.painter().hline(
            rect.x_range(),
            rect.bottom(),
            egui::Stroke::new(2.5_f32, Color32::LIGHT_GREEN),
        );
        ui.painter().vline(
            rect.left() + 2.0,
            rect.y_range(),
            egui::Stroke::new(4.0_f32, Color32::LIGHT_GREEN),
        );
    }
    if preparing {
        preparing_overlay(ui, response.response.rect);
    }
    if selected {
        ui.painter().rect_stroke(
            response.response.rect,
            0.0,
            egui::Stroke::new(1.0_f32, Color32::LIGHT_BLUE),
            egui::StrokeKind::Inside,
        );
    }
    RowResponse {
        clicked,
        cells: rects,
    }
}

pub(crate) const RAW_COLUMN: usize = 4;

/// Half a monospace character between raw columns.
pub(crate) fn raw_spacing(ui: &egui::Ui) -> f32 {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    ui.fonts(|fonts| fonts.glyph_width(&font, '0')) * 0.5
}

/// Blinking label on the start row: playback is requested but no audio has rendered yet.
fn preparing_overlay(ui: &egui::Ui, rect: egui::Rect) {
    let time = ui.input(|i| i.time);
    let pulse = 0.5 + 0.5 * (time * std::f64::consts::TAU).cos() as f32;
    let color = Color32::LIGHT_GREEN.gamma_multiply(0.35 + 0.55 * pulse);
    let galley = ui.painter().layout_no_wrap(
        "Preparing playback…".into(),
        egui::TextStyle::Body.resolve(ui.style()),
        color,
    );
    let text_rect = egui::Align2::LEFT_CENTER
        .anchor_size(rect.left_center() + egui::vec2(12.0, 0.0), galley.size());
    ui.painter()
        .rect_filled(text_rect.expand(3.0), 3.0, Color32::from_black_alpha(170));
    ui.painter().galley(text_rect.min, galley, color);
    ui.ctx().request_repaint();
}

pub(crate) fn raw_preview(event: &Event) -> String {
    if readable_meta(event) {
        return cat_smf_editor::score::event_content::decoded(event);
    }
    let prefix = raw_prefix(event);
    let bytes = hex(&event.data[..event.data.len().min(24)]);
    format!(
        "{prefix} {bytes}{}",
        if event.data.len() > 24 { " …" } else { "" }
    )
}

fn raw_prefix(event: &Event) -> String {
    if event.status == 0xff {
        format!("FF {:02X}", event.meta_type)
    } else {
        format!("{:02X}", event.status)
    }
}

fn raw_full(event: &Event) -> String {
    if readable_meta(event) {
        return cat_smf_editor::score::event_content::decoded(event);
    }
    format!("{} {}", raw_prefix(event), hex(&event.data))
}

fn readable_meta(event: &Event) -> bool {
    event.status == 0xff
        && ((1..=9).contains(&event.meta_type)
            || (event.meta_type == 0x7f && event.data.starts_with(b"\x7dcat-smf-editor\0")))
}

fn hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
