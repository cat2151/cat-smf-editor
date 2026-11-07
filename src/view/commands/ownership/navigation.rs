use super::*;

/// Screens whose commands every non-transient palette offers.
pub fn destinations(a: &App) -> Vec<Screen> {
    let mut out = vec![Screen::Settings, Screen::Program, Screen::Recent];
    if a.loaded.is_some() {
        out.extend([
            Screen::Main,
            Screen::Edit,
            Screen::Go,
            Screen::Tracks,
            Screen::Split,
            Screen::Play,
            Screen::Mixer,
            Screen::Layers,
            Screen::Effects,
            Screen::Timeline,
            Screen::Overview,
            Screen::Drums,
        ]);
        if crate::view::patterns::mode::song_window(a) {
            out.push(Screen::Song);
            if a.patterns.workspace.scene_window {
                out.push(Screen::Scene);
            }
        }
    }
    out
}

/// Screens that open over `current` instead of first returning to the Event list.
fn overlays(current: Screen) -> &'static [Screen] {
    match current {
        Screen::Timeline | Screen::Overview => &[Screen::Settings, Screen::Program, Screen::Play],
        Screen::Mixer => &[Screen::Effects],
        Screen::Scene | Screen::Song => &[Screen::Settings, Screen::Play],
        Screen::Settings => &[Screen::Program],
        Screen::Go => &[Screen::Tracks],
        _ => &[],
    }
}

/// Moves to `home` so its command can run; `false` leaves the user where navigation stopped.
pub fn navigate(a: &mut App, ctx: &egui::Context, current: Screen, home: Screen) -> bool {
    if current == home || home == Screen::Global {
        return true;
    }
    if let Some(p) = home.pane() {
        crate::view::patterns::workspace::focus(a, ctx, p);
        return true;
    }
    if overlays(current).contains(&home) {
        enter(a, ctx, home);
        return true;
    }
    if !leave_to_main(a, ctx) {
        a.load_error = Some("入力中の編集を確定または取消してから実行してください".into());
        return false;
    }
    if ctx.viewport_id() != egui::ViewportId::ROOT {
        crate::view::patterns::presentation::focus_root(a, ctx);
    }
    enter(a, ctx, home);
    root_owner(a) == home
}

fn leave_to_main(a: &mut App, ctx: &egui::Context) -> bool {
    loop {
        let screen = root_owner(a);
        if screen == Screen::Main {
            return true;
        }
        let Some(close) = screen
            .close()
            .and_then(|id| definitions(a, screen).into_iter().find(|c| c.id == id))
        else {
            return false;
        };
        (close.run)(a, ctx);
        if root_owner(a) == screen {
            return false;
        }
    }
}

fn enter(a: &mut App, ctx: &egui::Context, screen: Screen) {
    match screen {
        Screen::Settings | Screen::Program => {
            crate::view::comma_menu::open(a);
            a.comma_menu.program = screen == Screen::Program;
        }
        Screen::Edit => a.edit_menu.open = true,
        Screen::Go | Screen::Tracks => {
            if !a.track_view.go_open {
                a.track_view.origin_tick = crate::play::playback::location(a).0;
            }
            a.track_view.go_open = true;
            a.track_view.track_list = screen == Screen::Tracks;
        }
        Screen::Split => a.track_view.split_open = true,
        Screen::Play => {
            a.playback.open = true;
            a.playback.viewport = egui::ViewportId::ROOT;
        }
        Screen::Mixer => {
            crate::play::mixer::open(a);
            a.playback.mixer_master = false;
        }
        Screen::Layers => crate::view::layers::open(a),
        Screen::Effects => crate::play::mixer::open_effects(a, false),
        Screen::Recent => crate::view::recent::commands::open(a),
        Screen::Timeline => crate::view::piano_roll::open(a),
        Screen::Overview => crate::view::track_overview::open(a),
        Screen::Drums => crate::view::drums::open(a),
        _ => {}
    }
    let _ = ctx;
}
