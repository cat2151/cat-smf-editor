use super::*;

/// Answers the player's selections and notices when it has gone.
pub fn poll(app: &mut App, ctx: &egui::Context) {
    let Some(mut link) = app.player_selection.resident.take() else {
        return;
    };
    link.context
        .lock()
        .unwrap()
        .get_or_insert_with(|| ctx.clone());
    let mut browser_closed = None;
    let closed = loop {
        let event = match link.events.try_recv() {
            Ok(event) => event,
            Err(mpsc::TryRecvError::Empty) => break link.exited_early(),
            Err(mpsc::TryRecvError::Disconnected) => break Some(None),
        };
        match event {
            Event::Connected(sender) => {
                log("player connected");
                link.sender = Some(sender);
                if let Err(error) = link.show() {
                    break Some(Some(error));
                }
            }
            Event::Selected(selected) => {
                app.audition.browser_used();
                log(format_args!(
                    "selected {:?} {} {:?} ({} bytes); target {:?}",
                    selected.target,
                    selected.plugin.id,
                    selected.preset_name,
                    selected.state.len,
                    link.target
                ));
                let Some(sender) = &mut link.sender else {
                    log("selection before connection; dropped");
                    continue;
                };
                if let Err(error) = answer(app, selected, Some(&link.target), sender) {
                    break Some(Some(format!("playerに応答できませんでした: {error}")));
                }
            }
            Event::BrowserClosed(target) => {
                if target == link.target.open {
                    link.loading.observe(true);
                    if matches!(target, Open::Kit { .. }) {
                        app.player_selection.kit_open = false;
                    }
                    browser_closed = Some(target);
                }
            }
            Event::Panes(context, panes, query) => {
                app.config.select_browser_panes(context, panes, query)
            }
            Event::Cursor(context, patch) => app.config.select_patch_cursor(context, patch),
            Event::Closed(error) => break Some(error),
        }
    };
    match closed {
        None => {
            if link.sender.is_none() || link.loading.title().is_some() {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            app.player_selection.resident = Some(link);
            if let Some(target) = browser_closed
                && !crate::view::song_challenge::browser_closed(app, ctx, target)
                && matches!(target, Open::Kit { .. })
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
        Some(error) => {
            app.player_selection.kit_open = false;
            log(format_args!("link closed: {error:?}"));
            app.player_selection.open = true;
            app.player_selection.error = Some(match error {
                Some(error) => format!("cat-plugin-playerが止まりました: {error}"),
                None => "cat-plugin-player closed".into(),
            });
            ctx.request_repaint();
        }
    }
}
