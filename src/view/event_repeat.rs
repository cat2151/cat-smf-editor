//! Vertical A–B selection shared by event rows and their minimap.
use crate::app::Loaded;
use eframe::egui;

const COLOR: egui::Color32 = egui::Color32::from_rgb(0x66, 0xd9, 0xef);

pub(super) struct Selection {
    pub range: Option<(u64, u64)>,
    pub dragging: bool,
}

pub(super) fn gesture(
    ui: &egui::Ui,
    response: &egui::Response,
    tick: impl Fn(egui::Pos2) -> u64,
    y: impl Fn(u64) -> f32,
    range: Option<(u64, u64)>,
) -> Selection {
    let id = response.id.with("repeat-start");
    let (pressed, down, pos, origin, moved) = ui.input(|i| {
        (
            i.pointer.primary_pressed(),
            i.pointer.primary_down(),
            i.pointer.interact_pos(),
            i.pointer.press_origin(),
            i.pointer.is_decidedly_dragging(),
        )
    });
    if pressed
        && let Some(origin) = origin
        && response.rect.intersect(ui.clip_rect()).contains(origin)
        && ui.ctx().layer_id_at(origin) == Some(ui.layer_id())
    {
        ui.data_mut(|data| data.insert_temp(id, (tick(origin), false)));
    }
    let mut drag = ui.data_mut(|data| data.get_temp::<(u64, bool)>(id));
    if let Some((_, selecting)) = &mut drag {
        *selecting |= moved;
    }
    let preview = drag
        .filter(|(_, selecting)| *selecting)
        .zip(pos)
        .map(|((a, _), pos)| {
            let b = tick(pos);
            (a.min(b), a.max(b))
        });
    if down && let Some(drag) = drag {
        ui.data_mut(|data| data.insert_temp(id, drag));
    }
    if let Some((a, b)) = preview.or(range) {
        let painter = ui.painter_at(response.rect.intersect(ui.clip_rect()));
        let selected = egui::Rect::from_x_y_ranges(response.rect.x_range(), y(a)..=y(b));
        painter.rect_filled(selected, 0.0, COLOR.gamma_multiply(0.18));
        for (tick, label) in [(a, "A"), (b, "B")] {
            painter.hline(
                response.rect.x_range(),
                y(tick),
                egui::Stroke::new(1.0_f32, COLOR),
            );
            painter.text(
                egui::pos2(response.rect.right() - 4.0, y(tick)),
                egui::Align2::RIGHT_TOP,
                label,
                egui::FontId::monospace(12.0),
                COLOR,
            );
        }
    }
    if !down {
        ui.data_mut(|data| data.remove::<(u64, bool)>(id));
    }
    Selection {
        range: (!down).then_some(preview).flatten().filter(|(a, b)| a < b),
        dragging: down && preview.is_some(),
    }
}

pub(super) struct Rows<'a> {
    pub loaded: &'a Loaded,
    pub track: usize,
    pub note_mode: bool,
    pub generation: u64,
    pub range: Option<(u64, u64)>,
    pub stride: f32,
    pub count: usize,
}

pub(super) fn rows(
    ui: &egui::Ui,
    output: &egui::scroll_area::ScrollAreaOutput<()>,
    Rows {
        loaded,
        track,
        note_mode,
        generation,
        range,
        stride,
        count,
    }: Rows<'_>,
) -> (Option<(u64, u64)>, bool) {
    if count == 0 {
        return (None, false);
    }
    let rect = output.inner_rect;
    let response = ui.interact(
        rect,
        ui.id().with(("event-repeat", generation, track)),
        egui::Sense::hover(),
    );
    let top = rect.top() - output.state.offset.y;
    let tick = |pos: egui::Pos2| {
        let row = ((pos.y.clamp(rect.top(), rect.bottom()) - top) / stride).max(0.0) as usize;
        crate::cursor::split::row_tick_loaded(loaded, note_mode, track, row.min(count - 1))
    };
    let y = |tick| {
        top + crate::cursor::split::row_sounding_loaded(loaded, note_mode, track, tick) as f32
            * stride
    };
    let selection = gesture(ui, &response, tick, y, range);
    (selection.range, selection.dragging)
}
