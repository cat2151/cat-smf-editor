use super::*;
use crate::player::selection::{open, Browser};
use cat_smf_editor::{
    host::PluginKey,
    model::{Event, Music, Track},
    score::instrument::Patch as Settings,
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static LAUNCHED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Records the link name; the test connects as the player itself.
fn record(_: &Path, name: &str) -> std::io::Result<Option<Child>> {
    LAUNCHED.with(|l| l.borrow_mut().push(name.to_owned()));
    Ok(None)
}

fn launched() -> Vec<String> {
    LAUNCHED.with(|l| l.borrow().clone())
}

fn music() -> Music {
    let event = |tick, status, meta_type, data| Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data,
    };
    Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![
                    event(0, 0xff, 0x51, vec![7, 161, 32]),
                    event(960, 0xff, 0x2f, vec![]),
                ],
            },
            Track {
                events: vec![
                    event(0, 0x90, 0, vec![60, 100]),
                    event(480, 0x80, 0, vec![60, 0]),
                    event(960, 0xff, 0x2f, vec![]),
                ],
            },
        ],
    }
}

/// Polls like the GUI does until `done` holds.
fn poll_until(app: &mut App, ctx: &egui::Context, done: impl Fn(&App) -> bool) {
    let start = std::time::Instant::now();
    while !done(app) {
        assert!(start.elapsed() < Duration::from_secs(5), "timed out");
        poll(app, ctx);
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn connected(app: &App) -> bool {
    app.player_selection
        .resident
        .as_ref()
        .is_some_and(|link| link.sender.is_some())
}

#[test]
fn the_player_starts_once_and_later_requests_only_show_a_browser() {
    let ctx = egui::Context::default();
    let mut app = crate::view::tests::app(music());
    app.player_selection.executable = Some(std::env::current_exe().unwrap());
    app.player_selection.launch = Some(record);
    app.select_track(1);

    let t = std::time::Instant::now();
    open(&mut app, Browser::Patch);
    assert_eq!(app.player_selection.error, None);
    assert!(loading(&app));
    assert_eq!(launched().len(), 1);
    for time in [0.0, 1.0] {
        let output = crate::view::piano_roll::tests::at(&mut app, &ctx, time, vec![]);
        let center = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.text() == "now loading..." => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        });
        if time > 0.0 {
            assert!(
                center
                    .expect("loading overlay text")
                    .distance(egui::pos2(750.0, 450.0))
                    < 35.0
            );
        }
    }
    let (mut player_out, mut player_in) = resident_ipc::connect(&launched()[0]).unwrap();
    poll_until(&mut app, &ctx, connected);
    assert!(
        loading(&app),
        "connection alone does not mean the browser is visible"
    );
    app.player_selection
        .resident
        .as_mut()
        .unwrap()
        .loading
        .observe(true);
    assert!(!loading(&app));
    assert_eq!(
        player_in.receive().unwrap(),
        Some(ToPlayer::OpenFiltered {
            open: Open::Layer {
                track: 1,
                layer: 1,
                song: 0
            },
            context: pane_context(&app.player_selection.resident.as_ref().unwrap().target),
            panes: Default::default(),
            cursor: None,
            query: String::new()
        })
    );
    let first_open = t.elapsed();

    let state: Vec<u8> = (0..1_300_001u32).map(|i| (i % 251) as u8).collect();
    let shared = player_out.share(&state).unwrap();
    let t = std::time::Instant::now();
    player_out
        .send(&ToEditor::Selected(Selected {
            reference: Some(Box::new(cat_smf_editor::patch_reference::PatchReference {
                plugin: PluginKey {
                    format: "CLAP".into(),
                    id: "synth".into(),
                    name: "synth".into(),
                    vendor: String::new(),
                    bundle_path: "X:/plugins/synth.clap".into(),
                },
                source: cat_smf_editor::patch_reference::Source::Catalog {
                    path: "X:/patches/bass.preset".into(),
                    file: "X:/patches/bass.preset".into(),
                },
                display: "Bass".into(),
            })),
            target: Open::Layer {
                track: 1,
                layer: 1,
                song: 0,
            },
            plugin: PluginKey {
                format: "CLAP".into(),
                id: "synth".into(),
                name: "synth".into(),
                vendor: String::new(),
                bundle_path: "X:/plugins/synth.clap".into(),
            },
            preset_name: Some("Bass".into()),
            state: shared.reference.clone(),
        }))
        .unwrap();
    poll_until(&mut app, &ctx, |app| {
        app.loaded.as_ref().unwrap().patches[1] != Settings::default()
    });
    assert_eq!(
        player_in.receive().unwrap(),
        Some(ToPlayer::Applied(Ok(())))
    );
    let applied = t.elapsed();
    drop(shared);
    let patches = &app.loaded.as_ref().unwrap().patches;
    assert_eq!(
        patches[1].layers[0].instrument.as_ref().unwrap().state,
        state
    );

    let t = std::time::Instant::now();
    open(&mut app, Browser::Effect);
    assert!(loading(&app));
    assert_eq!(
        player_in.receive().unwrap(),
        Some(ToPlayer::OpenFiltered {
            open: Open::Effect {
                track: 1,
                slot: None
            },
            context: pane_context(&app.player_selection.resident.as_ref().unwrap().target),
            panes: Default::default(),
            cursor: None,
            query: String::new()
        })
    );
    let second_open = t.elapsed();
    assert_eq!(launched().len(), 1, "a running player is not started again");
    println!(
        "open (first) {first_open:?}, selection applied {applied:?}, open (second) {second_open:?}"
    );

    // The player going away is reported, and the next request starts a new one.
    drop((player_out, player_in));
    poll_until(&mut app, &ctx, |app| {
        app.player_selection.resident.is_none()
    });
    assert!(!loading(&app));
    assert!(app
        .player_selection
        .error
        .as_ref()
        .unwrap()
        .contains("closed"));
    open(&mut app, Browser::Patch);
    assert_eq!(launched().len(), 2);
    drop(resident_ipc::connect(&launched()[1]));
}

#[test]
fn a_player_that_exits_before_connecting_is_reported() {
    fn exits(_: &Path, _: &str) -> std::io::Result<Option<Child>> {
        Command::new("cmd").args(["/c", "exit 3"]).spawn().map(Some)
    }
    let ctx = egui::Context::default();
    let mut app = crate::view::tests::app(music());
    app.player_selection.executable = Some(std::env::current_exe().unwrap());
    app.player_selection.launch = Some(exits);
    app.select_track(1);
    open(&mut app, Browser::Patch);
    poll_until(&mut app, &ctx, |app| {
        app.player_selection.resident.is_none()
    });
    let error = app.player_selection.error.as_deref().unwrap();
    assert!(error.contains("exited before connecting"), "{error}");
}

/// Titles of the visible, titled top-level windows of `process`.
fn titles(process: u32) -> Vec<String> {
    use std::ffi::c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(
            callback: unsafe extern "system" fn(*mut c_void, isize) -> i32,
            data: isize,
        ) -> i32;
        fn GetWindowThreadProcessId(window: *mut c_void, process: *mut u32) -> u32;
        fn IsWindowVisible(window: *mut c_void) -> i32;
        fn GetWindowTextW(window: *mut c_void, text: *mut u16, len: i32) -> i32;
    }
    unsafe extern "system" fn collect(window: *mut c_void, data: isize) -> i32 {
        let (process, found) = unsafe { &mut *(data as *mut (u32, Vec<String>)) };
        let mut owner = 0;
        unsafe { GetWindowThreadProcessId(window, &mut owner) };
        if owner == *process && unsafe { IsWindowVisible(window) } != 0 {
            let mut text = [0u16; 256];
            let len = unsafe { GetWindowTextW(window, text.as_mut_ptr(), 256) };
            if len > 0 {
                found.push(String::from_utf16_lossy(&text[..len as usize]));
            }
        }
        1
    }
    let mut data = (process, Vec::new());
    unsafe { EnumWindows(collect, &mut data as *mut _ as isize) };
    data.1
}

#[test]
#[ignore = "needs RESIDENT_PLAYER (cat-plugin-player.exe built from the sibling repo); shows its browser"]
fn the_real_player_starts_once_and_ends_with_the_editor() {
    use std::time::Instant;
    let ctx = egui::Context::default();
    let mut app = crate::view::tests::app(music());
    app.player_selection.executable = Some(std::env::var_os("RESIDENT_PLAYER").unwrap().into());
    app.select_track(1);
    let pid = |app: &App| {
        let link = app.player_selection.resident.as_ref().unwrap();
        link.child.as_ref().unwrap().id()
    };
    let shown_after = |app: &mut App, browser, title: &str| {
        let start = Instant::now();
        open(app, browser);
        assert_eq!(app.player_selection.error, None);
        assert!(loading(app));
        let process = pid(app);
        while !titles(process).iter().any(|t| t == title) {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "{:?}",
                titles(process)
            );
            poll(app, &ctx);
            std::thread::sleep(Duration::from_millis(2));
        }
        refresh_browser_activity(app);
        assert!(
            !loading(app),
            "the visible browser must dismiss the loading overlay"
        );
        start.elapsed()
    };
    let first = shown_after(&mut app, Browser::Patch, "Browse patches");
    let process = pid(&app);
    std::thread::sleep(Duration::from_millis(500));
    let second = shown_after(&mut app, Browser::Effect, "Browse effects");
    assert_eq!(pid(&app), process, "the running player is reused");
    let mut child = app
        .player_selection
        .resident
        .as_mut()
        .unwrap()
        .child
        .take()
        .unwrap();
    let closed = Instant::now();
    drop(app);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(
            closed.elapsed() < Duration::from_secs(10),
            "player still running"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    println!(
        "browser shown {first:?} after the first request, {second:?} after the second; player exited {:?} after the editor closed ({status})",
        closed.elapsed()
    );
    assert!(status.success());
}

/// Visible top-level windows of `process` titled `title`.
fn window(process: u32, title: &str) -> Option<usize> {
    use std::ffi::c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(
            callback: unsafe extern "system" fn(*mut c_void, isize) -> i32,
            data: isize,
        ) -> i32;
        fn GetWindowThreadProcessId(window: *mut c_void, process: *mut u32) -> u32;
        fn IsWindowVisible(window: *mut c_void) -> i32;
        fn GetWindowTextW(window: *mut c_void, text: *mut u16, len: i32) -> i32;
    }
    unsafe extern "system" fn collect(window: *mut c_void, data: isize) -> i32 {
        let (process, found) = unsafe { &mut *(data as *mut (u32, Vec<(usize, String)>)) };
        let mut owner = 0;
        unsafe { GetWindowThreadProcessId(window, &mut owner) };
        if owner == *process && unsafe { IsWindowVisible(window) } != 0 {
            let mut text = [0u16; 256];
            let len = unsafe { GetWindowTextW(window, text.as_mut_ptr(), 256) };
            found.push((
                window as usize,
                String::from_utf16_lossy(&text[..len as usize]),
            ));
        }
        1
    }
    let mut data = (process, Vec::new());
    unsafe { EnumWindows(collect, &mut data as *mut _ as isize) };
    data.1
        .into_iter()
        .find(|(_, t): &(usize, String)| t == title)
        .map(|(w, _)| w)
}

fn press(window: usize, vk: usize, scan: isize) {
    use std::ffi::c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn PostMessageW(window: *mut c_void, message: u32, w: usize, l: isize) -> i32;
    }
    unsafe {
        PostMessageW(window as *mut c_void, 0x0100, vk, 1 | (scan << 16));
        PostMessageW(
            window as *mut c_void,
            0x0101,
            vk,
            1 | (scan << 16) | (0xC000_0000u32 as isize),
        );
    }
}

#[test]
#[ignore = "needs RESIDENT_PLAYER and the installed plugins; drives the real browser"]
fn real_patch_choices_each_reach_the_track() {
    use std::time::Instant;
    let ctx = egui::Context::default();
    let mut app = crate::view::tests::app(music());
    app.player_selection.executable = Some(std::env::var_os("RESIDENT_PLAYER").unwrap().into());
    app.select_track(1);
    let preset = |app: &App| {
        app.loaded.as_ref().unwrap().patches[1].layers[0]
            .instrument
            .as_ref()
            .and_then(|i| i.preset_name.clone())
    };
    let mut seen = Vec::new();
    for round in 0..8 {
        if round % 3 == 0 {
            // The patch name button, pressed again.
            crate::player::bulk::request(&mut app);
            assert_eq!(app.player_selection.error, None);
        }
        let process = app
            .player_selection
            .resident
            .as_ref()
            .unwrap()
            .child
            .as_ref()
            .unwrap()
            .id();
        let before = preset(&app);
        let start = Instant::now();
        let mut last_press = None::<Instant>;
        while preset(&app) == before {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "round {round}: no new patch reached the track; seen={seen:?} error={:?}",
                app.player_selection.error
            );
            // Until the catalog is listed a press does nothing, so keep pressing.
            let due = last_press.is_none_or(|t| t.elapsed() > Duration::from_secs(3));
            if let Some(w) = window(process, "Browse patches").filter(|_| due) {
                press(w, 0x52, 0x13);
                last_press = Some(Instant::now());
            }
            poll(&mut app, &ctx);
            std::thread::sleep(Duration::from_millis(5));
        }
        seen.push((preset(&app), start.elapsed()));
    }
    println!("{seen:#?}");
}

#[path = "resident_panes_tests.rs"]
mod panes;
