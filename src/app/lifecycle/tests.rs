use super::*;

fn new_song() -> App {
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    assert!(app.install_new_song());
    app
}

fn complete(app: &mut App, ctx: &egui::Context, drawn: bool) {
    let _ = ctx.run(Default::default(), |ctx| {
        app.frame_started(ctx);
        app.frame_completed(ctx, drawn);
    });
}

#[test]
fn idle_is_an_edge_event_not_a_callback_on_every_frame() {
    let mut state = State::default();
    assert_eq!(state.observe(false), None);
    assert_eq!(state.observe(true), Some(Event::EnteredIdle));
    assert_eq!(state.observe(true), None);
    assert_eq!(state.observe(false), None);
    assert_eq!(state.observe(true), Some(Event::EnteredIdle));
}

#[test]
fn drawing_without_readiness_does_not_offer_an_edit_style() {
    let mut app = new_song();
    let ctx = egui::Context::default();
    app.startup.pending = true;
    complete(&mut app, &ctx, true);
    complete(&mut app, &ctx, true);
    assert!(app.edit_style.is_none());
    app.startup.pending = false;
    app.help_open = true;
    complete(&mut app, &ctx, true);
    assert!(app.edit_style.is_none());
    app.help_open = false;
    complete(&mut app, &ctx, true);
    assert!(app.edit_style.is_none());
    complete(&mut app, &ctx, true);
    assert!(app.edit_style.is_some());
}

#[test]
fn readiness_without_event_list_does_not_emit_and_a_layout_retry_does_not_dispatch() {
    let mut app = new_song();
    let ctx = egui::Context::default();
    complete(&mut app, &ctx, false);
    complete(&mut app, &ctx, false);
    assert!(app.edit_style.is_none());
    let _ = ctx.run(Default::default(), |ctx| {
        app.frame_completed(ctx, true);
        app.frame_started(ctx);
        assert!(app.edit_style.is_none());
    });
    complete(&mut app, &ctx, true);
    assert!(app.edit_style.is_some());
}

#[test]
fn an_existing_mid_with_initial_contents_does_not_need_guidance() {
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let ctx = egui::Context::default();
    complete(&mut app, &ctx, true);
    complete(&mut app, &ctx, true);
    assert!(app.edit_style.is_none());
}

#[test]
fn editing_before_idle_cancels_guidance_and_dismissed_guidance_is_not_repeated() {
    let mut app = new_song();
    let ctx = egui::Context::default();
    app.add_track().unwrap();
    complete(&mut app, &ctx, true);
    complete(&mut app, &ctx, true);
    assert!(app.edit_style.is_none());
    assert!(app.install_new_song());
    complete(&mut app, &ctx, true);
    complete(&mut app, &ctx, true);
    assert!(app.edit_style.take().is_some());
    complete(&mut app, &ctx, true);
    complete(&mut app, &ctx, true);
    assert!(app.edit_style.is_none());
}

#[test]
fn readiness_lost_before_dispatch_and_an_old_song_notification_are_discarded() {
    let mut app = new_song();
    let ctx = egui::Context::default();
    complete(&mut app, &ctx, true);
    app.help_open = true;
    complete(&mut app, &ctx, true);
    assert!(app.edit_style.is_none());
    app.help_open = false;
    complete(&mut app, &ctx, true);
    app.song_id += 1;
    complete(&mut app, &ctx, true);
    assert!(app.edit_style.is_none());
}
