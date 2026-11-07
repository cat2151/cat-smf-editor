use super::*;

pub fn edit(a: &mut App, ctx: &egui::Context, p: Pane) {
    match current(a, p) {
        Some(PatternOrScene::Pattern(id)) => {
            let r = show_body(a, p, id);
            if r.is_ok() {
                a.patterns.workspace.return_to = Some(p);
                presentation::focus_root(a, ctx);
            }
            result(a, r);
        }
        Some(PatternOrScene::Scene(id)) => {
            let r = open_scene(a, id);
            if r.is_ok() && p == Pane::Song {
                timing::bind_scene(a, id);
            }
            result(a, r);
            focus(a, ctx, Pane::Scene);
        }
        None => {}
    }
}
/// The chosen part opens independently of any placement; Escape returns to this pane.
/// A drum pattern opens in the drum editor and an empty one in MML input.
pub fn edit_choice(a: &mut App, ctx: &egui::Context, p: Pane) {
    match a.patterns.workspace.selection(p).choice {
        Some(PatternOrScene::Pattern(id)) => {
            let r = library::edit(a, id);
            if r.is_ok() {
                a.patterns.workspace.return_to = Some(p);
                presentation::focus_root(a, ctx);
                let l = a.loaded.as_ref().unwrap();
                if !crate::view::drums::is_open(a) && super::super::beacon::empty(l, id) {
                    crate::view::drums::input(a, ctx);
                }
            }
            result(a, r);
        }
        Some(PatternOrScene::Scene(id)) => {
            let r = open_scene(a, id);
            result(a, r);
            focus(a, ctx, Pane::Scene);
        }
        None => result(a, Err(message::SELECT_A_PATTERN_OR_SCENE.into())),
    }
}
/// Open the body editor, bound to the placement under the selected cell.
fn show_body(a: &mut App, p: Pane, id: u64) -> Result<(), String> {
    let lane = lane(a, p);
    let placement = a
        .loaded
        .as_ref()
        .and_then(|l| scene::read(&l.music).ok())
        .and_then(|c| {
            let selection = a.patterns.workspace.selection(p);
            let measure = model::covering(&c, scene_id(a, p), lane?, selection.measure)?.1;
            if p == Pane::Scene {
                c.scenes
                    .iter()
                    .find(|s| Some(s.id) == a.patterns.workspace.scene)?
                    .cells
                    .iter()
                    .find(|cell| Some(cell.lane) == lane && cell.measure + 1 == measure)
                    .map(|cell| cell.id)
            } else {
                c.song
                    .iter()
                    .find(|cell| Some(cell.lane) == lane && cell.measure == measure)
                    .map(|cell| cell.id)
            }
        });
    library::edit(a, id)?;
    if let Some(c) = &mut a.patterns.editing {
        c.placement = placement;
    }
    if let Some(placement) = placement {
        timing::bind_pattern(a, p, id, placement);
    }
    Ok(())
}
/// A click on a placed pattern opens its body editor and focus stays here;
/// a click on a placed scene opens it in the scene window, which takes focus.
pub fn click_body(a: &mut App, p: Pane) {
    let r = match current(a, p) {
        Some(PatternOrScene::Pattern(id)) => show_body(a, p, id),
        Some(PatternOrScene::Scene(id)) => {
            let r = open_scene(a, id);
            if r.is_ok() && p == Pane::Song {
                timing::bind_scene(a, id);
            }
            r
        }
        None => return,
    };
    result(a, r);
}
