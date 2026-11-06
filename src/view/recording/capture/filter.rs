//! A container filter shared by the list, keyboard selection and audition chain.
use super::*;

pub(super) fn owns_keys(ctx: &egui::Context) -> bool {
    let combo = ctx.data(|d| d.get_temp::<egui::Id>(egui::Id::new("capture-filter-focus")));
    ctx.memory(|m| m.any_popup_open() || combo.is_some_and(|id| m.has_focus(id)))
}

pub(super) fn refresh(b: &mut Browser) {
    let kept = b.phrases.get(b.selected).map(|p| (p.pattern, p.range.0));
    b.phrases = b
        .all_phrases
        .iter()
        .filter(|p| b.container.is_none_or(|id| p.container == id))
        .cloned()
        .collect();
    b.selected = kept
        .and_then(|k| b.phrases.iter().position(|p| (p.pattern, p.range.0) == k))
        .unwrap_or(0);
}

pub(super) fn set(a: &mut App, s: &mut Session, container: Option<u64>) {
    let listening = auditioning(a, s);
    let Some(b) = s.capture.as_mut() else { return };
    if b.container == container {
        return;
    }
    b.container = container;
    b.playback = None;
    refresh(b);
    if listening {
        crate::play::transport::stop(a);
    }
}

pub(super) fn dropdown(ui: &mut egui::Ui, s: &Session) -> Option<rows::Click> {
    let b = s.capture.as_ref()?;
    let mut selected = b.container;
    let label = |id| {
        s.sounds
            .iter()
            .find(|sound| sound.container == Some(id))
            .map_or_else(|| format!("Container {id}"), |sound| sound.name.clone())
    };
    let mut containers: Vec<_> = b.all_phrases.iter().map(|p| p.container).collect();
    // Keep the selected container available even after its history is cleared.
    containers.extend(b.container);
    containers.sort_unstable();
    containers.dedup();
    let combo = egui::ComboBox::from_id_salt("capture-container")
        .selected_text(selected.map_or_else(|| "全container".into(), label))
        .width(160.0)
        .show_ui(ui, |ui| {
            let _all = ui.selectable_value(&mut selected, None, "全container");
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("capture-filter-all"), _all.rect));
            for id in containers {
                let _option = ui.selectable_value(&mut selected, Some(id), label(id));
                #[cfg(test)]
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new(("capture-filter-option", id)), _option.rect)
                });
            }
        });
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new("capture-filter-focus"), combo.response.id));
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new("capture-filter"), combo.response.rect));
    (selected != b.container).then_some(rows::Click::Filter(selected))
}
