use super::*;
pub fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if a.loaded.is_none() || s == Screen::ArrangementInput {
        return;
    }
    let p = match s {
        Screen::Scene => Some(Pane::Scene),
        Screen::Song => Some(Pane::Song),
        _ => None,
    };
    if let Some(p) = p {
        let cells = a.patterns.workspace.selection(p).cells(p);
        if p == Pane::Song {
            super::timeline::collect(out);
            if !cells {
                out.push(Command::new(
                    s,
                    "arrangement-delete-pattern",
                    "Delete candidate pattern",
                    &[(Key::Delete, Modifiers::SHIFT)],
                    |a, _| {
                        if let Some(PatternOrScene::Pattern(id)) =
                            a.patterns.workspace.song_cell.choice
                        {
                            let r = super::deletion::pattern(a, id);
                            result(a, r);
                        }
                    },
                ));
            }
            out.push(Command::plain(
                s,
                "export-performance",
                "Export performance SMF",
                &[],
                |a, _| crate::view::files::performance::dialog(a),
            ));
        }

        if cells {
            for (id, key, letter, rows, measures) in [
                ("left", Key::ArrowLeft, Key::H, 0, -1),
                ("right", Key::ArrowRight, Key::L, 0, 1),
                ("up", Key::ArrowUp, Key::K, -1, 0),
                ("down", Key::ArrowDown, Key::J, 1, 0),
            ] {
                let keys = [key, letter];
                out.push(
                    Command::plain(
                        s,
                        format!("arrangement-{id}"),
                        "Cell",
                        &keys[..if p == Pane::Song { 2 } else { 1 }],
                        move |a, _| movement(a, p, rows, measures),
                    )
                    .repeating(),
                );
                out.push(Command::new(
                    s,
                    format!("arrangement-move-{id}"),
                    "Move placement",
                    &[(key, Modifiers::CTRL)],
                    move |a, _| move_placement(a, p, rows, measures),
                ));
            }
        }
        let candidate_keys = if p == Pane::Song {
            if cells {
                vec![]
            } else {
                vec![
                    (Key::K, -1),
                    (Key::ArrowUp, -1),
                    (Key::J, 1),
                    (Key::ArrowDown, 1),
                ]
            }
        } else {
            vec![(Key::PageUp, -1), (Key::PageDown, 1)]
        };
        for (key, delta) in candidate_keys {
            out.push(Command::plain(
                s,
                format!("arrangement-part-{key:?}"),
                "Pattern / scene",
                &[key],
                move |a, _| candidate(a, p, delta),
            ));
        }
        out.push(Command::plain(
            s,
            "arrangement-place-selected",
            if cells { "Put" } else { "Select candidate" },
            if p == Pane::Song {
                &[Key::F, Key::Enter]
            } else {
                &[Key::Enter]
            },
            move |a, _| confirm(a, p),
        ));
        out.push(Command::plain(
            s,
            "arrangement-edit",
            if cells {
                "Edit cell contents"
            } else {
                "Edit candidate"
            },
            &[Key::E],
            move |a, c| {
                if p == Pane::Song && !cells {
                    edit_choice(a, c, p)
                } else {
                    edit(a, c, p)
                }
            },
        ));
        if p == Pane::Scene || !cells {
            out.push(Command::new(
                s,
                "arrangement-edit-part",
                "Edit candidate",
                &[(Key::E, Modifiers::SHIFT)],
                move |a, c| edit_choice(a, c, p),
            ));
        }
        out.push(Command::plain(
            s,
            "arrangement-create-pattern",
            "New pattern",
            &[Key::N],
            crate::view::patterns::library::add,
        ));
        if cells {
            out.push(Command::plain(
                s,
                "arrangement-remove",
                "Remove placement",
                &[Key::Delete],
                move |a, _| {
                    let r = place(a, p, None);
                    result(a, r);
                },
            ));
            out.push(Command::new(
                s,
                "arrangement-comment-out",
                "Comment out placement",
                &[(Key::Slash, Modifiers::CTRL)],
                move |a, _| comment_out(a, p),
            ));
        }
        out.push(Command::plain(
            s,
            "arrangement-play",
            "Play",
            &[Key::Space],
            move |a, _| play(a, p),
        ));
        out.push(Command::new(
            s,
            "stop",
            "Stop",
            &[(Key::Space, Modifiers::SHIFT)],
            |a, _| crate::play::transport::stop(a),
        ));
        out.push(Command::new(
            s,
            "undo",
            "Undo",
            &[(Key::Z, Modifiers::CTRL)],
            |a, _| {
                let r = a.undo_music();
                result(a, r);
            },
        ));
        out.push(Command::plain(
            s,
            "play-menu",
            "再生menu",
            &[Key::P],
            |a, _| a.playback.open = true,
        ));
        if p == Pane::Song {
            out.push(Command::plain(
                s,
                "scene-window",
                "Scene window",
                &[Key::S],
                |a, _| a.patterns.workspace.scene_window ^= true,
            ));
        }
        out.push(Command::plain(
            s,
            "help",
            "Help",
            &[Key::F1],
            move |a, _| a.help_open = !a.help_open,
        ));
        out.push(Command::plain(
            s,
            "arrangement-close",
            "Close",
            &[Key::Escape],
            move |a, c| {
                if p == Pane::Song && a.patterns.workspace.song_cell.placing {
                    a.patterns.workspace.song_cell.placing = false;
                    a.patterns.workspace.song_cell.reveal_choice = true;
                    return;
                }
                crate::play::transport::stop(a);
                mode::close_workspace(a);
                presentation::focus_root(a, c);
            },
        ));
    }
    if !matches!(s, Screen::Main | Screen::Scene | Screen::Song) {
        return;
    }
    out.push(Command::plain(
        s,
        "scene-structure",
        "Scene measures / meter",
        &[],
        |a, c| open_dialog(a, c, dialog::Purpose::ResizeScene, Pane::Scene),
    ));
    out.push(Command::plain(
        s,
        "arrangement-create-scene",
        "Create scene",
        &[],
        move |a, c| open_dialog(a, c, dialog::Purpose::Create, Pane::Scene),
    ));
    out.push(Command::plain(
        s,
        "arrangement-edit-scene",
        "Edit scene",
        &[],
        |a, c| open_dialog(a, c, dialog::Purpose::Scene, Pane::Scene),
    ));
    out.push(Command::plain(
        s,
        "arrangement-play-song",
        "Play song",
        &[],
        |a, _| play(a, Pane::Song),
    ));
    out.push(Command::plain(
        s,
        "arrangement-play-scene",
        "Play scene",
        &[],
        |a, _| play(a, Pane::Scene),
    ));
    if s == Screen::Scene {
        out.push(Command::plain(
            s,
            "arrangement-delete-scene",
            "Delete scene",
            &[],
            |a, _| {
                let id = a.patterns.workspace.scene;
                let r = id
                    .ok_or("Select a scene".into())
                    .and_then(|id| mutate(a, |m| model::delete_scene(m, id)));
                result(a, r);
            },
        ));
    }
    if s == Screen::Main
        && let Some(p) = a.patterns.workspace.return_to
    {
        out.push(Command::plain(
            s,
            "arrangement-return",
            "Return to placement",
            &[Key::Escape],
            move |a, c| {
                a.patterns.workspace.return_to = None;
                focus(a, c, p);
            },
        ));
    }
}
pub(super) fn confirm(a: &mut App, p: Pane) {
    let s = a.patterns.workspace.selection_mut(p);
    if p == Pane::Song && !s.placing {
        if s.choice.is_some() {
            s.placing = true;
        }
    } else {
        place_selected(a, p);
    }
}
/// An empty cell is left alone, so nothing enters the undo history.
pub(super) fn comment_out(a: &mut App, p: Pane) {
    let Some(l) = &a.loaded else { return };
    let Ok(c) = scene::read(&l.music) else { return };
    let scene = scene_id(a, p);
    let Some(lane) = lane(a, p) else { return };
    let measure = a.patterns.workspace.selection(p).measure;
    if model::covering(&c, scene, lane, measure).is_none() {
        return;
    }
    let r = mutate(a, |m| model::toggle(m, scene, lane, measure));
    result(a, r);
}
fn move_placement(a: &mut App, p: Pane, rows: isize, measures: isize) {
    let Some(l) = &a.loaded else { return };
    let Ok(c) = scene::read(&l.music) else { return };
    let scene = scene_id(a, p);
    let Some(lane) = lane(a, p) else { return };
    let s = a.patterns.workspace.selection(p);
    let Some(j) = s
        .row
        .checked_add_signed(rows)
        .filter(|&j| j < super::rows(&c, scene))
    else {
        return;
    };
    let Some(measure) = s
        .measure
        .checked_add_signed(measures as i64)
        .filter(|m| *m > 0)
    else {
        return;
    };
    let start = model::covering(&c, scene, lane, s.measure).map_or(s.measure, |(_, start)| start);
    let from = (lane, start);
    let to = start
        .checked_add_signed(measures as i64)
        .filter(|m| *m > 0)
        .unwrap_or(measure);
    let r = mutate(a, |m| {
        let lane = materialize(m, scene, j)?;
        model::move_part(m, scene, from, (lane, to))
    });
    if r.is_ok() {
        let s = a.patterns.workspace.selection_mut(p);
        s.row = j;
        s.measure = to;
    }
    result(a, r);
}
fn candidate(a: &mut App, p: Pane, delta: isize) {
    let all = choices(a, p);
    let s = a.patterns.workspace.selection_mut(p);
    s.reveal_choice = true;
    let all = all
        .into_iter()
        .filter(|(_, label)| s.search.matches_text(label))
        .collect::<Vec<_>>();
    let i = all
        .iter()
        .position(|(part, _)| Some(*part) == s.choice)
        .unwrap_or(0);
    s.choice = all
        .get(
            i.saturating_add_signed(delta)
                .min(all.len().saturating_sub(1)),
        )
        .map(|(part, _)| *part);
}
fn movement(a: &mut App, p: Pane, rows: isize, measures: isize) {
    let Some(l) = &a.loaded else { return };
    let Ok(c) = scene::read(&l.music) else { return };
    let id = scene_id(a, p);
    let count = super::rows(&c, id);
    let s = a.patterns.workspace.selection_mut(p);
    s.row = s
        .row
        .saturating_add_signed(rows)
        .min(count.saturating_sub(1));
    s.measure = s.measure.saturating_add_signed(measures as i64).max(1);
    if let Some(scene) = id.and_then(|id| c.scenes.iter().find(|s| s.id == id)) {
        s.measure = s.measure.min(scene.meters.len() as u64);
    }
}
