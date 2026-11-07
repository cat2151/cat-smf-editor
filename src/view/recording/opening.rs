use super::*;

/// The conductor's key at tick 0.
fn song_key(music: &cat_smf_editor::model::Music) -> recording::Key {
    recording::Key::from_signature(music.tracks.first().and_then(|t| {
        t.events
            .iter()
            .take_while(|e| e.tick == 0)
            .find_map(KeySignature::from_event)
    }))
}

pub(super) fn open(a: &mut App, ctx: &egui::Context) -> Result<(), String> {
    let pane = super::super::patterns::workspace::pane(ctx);
    open_from(a, ctx, ctx.viewport_id(), pane, true)
}

/// Opens as Shift+R in `origin`, which is `pane` when that is a pattern window.
pub(super) fn open_from(
    a: &mut App,
    ctx: &egui::Context,
    origin: egui::ViewportId,
    pane: Option<Pane>,
    stop_playback: bool,
) -> Result<(), String> {
    if let Some(s) = a.patterns.recording.session.as_mut() {
        s.origin = origin;
        restore(ctx);
        return Ok(());
    }
    if modal::is_open(a) {
        return Ok(());
    }
    let loaded = a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?;
    let c = Catalog::read(&loaded.music)?;
    let key = song_key(&loaded.music);
    let (mut settings, measures) = a.config.keyboard_settings.unwrap_or_else(|| {
        (
            Settings::new(
                key,
                setup::tempo(super::super::patterns::timing::bpm(a).round() as i32),
            ),
            4,
        )
    });
    if settings.key() != key {
        settings.set_key(key);
    }
    Recorder::new(loaded.music.division, settings)?;
    let looper = || {
        if a.config.phrase_loop {
            phrase::Looper::Waiting
        } else {
            phrase::Looper::Off
        }
    };
    let mut sounds = c
        .tracks
        .iter()
        .filter(|(_, t)| c.sources.contains(t))
        .map(|(&id, &track)| Sound {
            container: Some(id),
            track,
            patch: loaded.patches[track].clone(),
            name: cat_smf_editor::score::pattern::container_name(&loaded.music, id, track),
            live: live::Live::default(),
            looper: looper(),
            kit_bank: 0,
            kit_base: 36,
        })
        .collect::<Vec<_>>();
    sounds.sort_by_key(|s| s.track);
    let target = if let Some(pane) = pane {
        match a.patterns.workspace.selection(pane).choice {
            Some(PatternOrScene::Pattern(id)) => c.patterns.get(&id).map(|p| p.container),
            _ => a.patterns.container,
        }
    } else {
        c.tracks
            .iter()
            .find_map(|(&id, &track)| (track == a.track).then_some(id))
    };
    if sounds.is_empty() {
        sounds.push(Sound {
            container: None,
            track: loaded.music.tracks.len(),
            patch: Patch::default(),
            name: "Init Saw".into(),
            live: live::Live::default(),
            looper: looper(),
            kit_bank: 0,
            kit_base: 36,
        });
    }
    let selected = sounds.iter().position(|s| s.container == target);
    let index = selected.unwrap_or(0);
    if let Some(v) = sounds[index]
        .container
        .and_then(|id| a.config.keyboard_voicings.get(&id))
    {
        settings.set_voicing(*v);
    }
    let division = loaded.music.division;
    if stop_playback {
        crate::play::transport::stop(a);
    }
    super::super::piano_roll::cancel_keyboard(a, ctx);
    let mut session = Session {
        origin,
        #[cfg(windows)]
        owner: super::super::analysis::owner::editor(),
        #[cfg(not(windows))]
        owner: None,
        owned: vec![],
        division,
        clock: Instant::now(),
        sounds,
        selected: index,
        settings,
        measures,
        item: setup::Item::Range,
        settings_open: false,
        piano_open: true,
        raise: false,
        focused: true,
        minimized: false,
        companions: companions::Places::default(),
        mouse: [None; 2],
        recorder: None,
        sweep: None,
        click: None,
        generation: a.generation,
        pending: capture::Pending::new(),
        capture: None,
        error: None,
        clicked: None,
        new_sound: None,
        kit_creation: None,
        handled: vec![],
        position: companions::saved(a, companions::KEYBOARD).unwrap_or_else(|| {
            let screen = egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                ctx.input(|i| i.viewport().monitor_size)
                    .unwrap_or(egui::vec2(1920.0, 1080.0)),
            );
            window::place(
                screen,
                &window::covers(a, ctx),
                window::SIZE + window::FRAME,
            )
        }),
        down: pads::key_down,
    };
    session.item = setup::first(&session);
    // Pads come first; a sound that cannot play opens the settings beside the error.
    session.settings_open = !play::start(a, &mut session);
    a.patterns.recording.session = Some(session);
    modal::mark_owned(ctx);
    modal::clear(ctx);
    ctx.request_repaint();
    Ok(())
}
