use super::*;
pub fn collect(app: &App, screen: Screen, out: &mut Vec<Command>) {
    if screen != Screen::Singer {
        return;
    }
    for (key, modifiers, action) in crate::view::list_navigation::bindings() {
        out.push(
            Command::new(
                screen,
                match key {
                    Key::ArrowUp => "singer.prev".into(),
                    Key::ArrowDown => "singer.next".into(),
                    _ => format!("singer.nav-{key:?}"),
                },
                &[(key, modifiers)],
                move |a, _| {
                    let len = filtered(a).len();
                    if let Some(p) = &mut a.voicevox.singer {
                        p.selected = action.apply(p.selected, len);
                        p.reveal = true;
                    }
                },
            )
            .repeating(),
        );
    }
    out.push(Command::plain(
        screen,
        "singer.confirm",
        &[Key::Enter],
        confirm,
    ));
    out.push(Command::plain(
        screen,
        "singer.cancel",
        &[Key::Escape],
        |a, _| close(a),
    ));
    out.push(Command::plain(screen, "singer.disable", &[], |a, _| {
        apply(a, None)
    }));
    if app.voicevox.catalog.error.is_some() {
        out.push(Command::plain(screen, "singer.retry", &[], |a, _| {
            preview::retry(a)
        }));
    }
}

pub fn keep_ime(app: &mut App, ctx: &egui::Context) {
    let Some(p) = &mut app.voicevox.singer else {
        return;
    };
    let events = ctx.input(|i| i.events.clone());
    let mut ime = p.composing;
    for e in &events {
        if let egui::Event::Ime(e) = e {
            ime = true;
            match e {
                egui::ImeEvent::Preedit(text) => p.composing = !text.is_empty(),
                egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => p.composing = false,
                _ => {}
            }
        }
    }
    if ime
        || events
            .iter()
            .any(|e| matches!(e, egui::Event::Text(_) | egui::Event::Paste(_)))
    {
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                !matches!(
                    e,
                    egui::Event::Key {
                        key: Key::Enter
                            | Key::Escape
                            | Key::ArrowUp
                            | Key::ArrowDown
                            | Key::PageUp
                            | Key::PageDown
                            | Key::Home
                            | Key::End,
                        ..
                    }
                )
            })
        });
    }
}
