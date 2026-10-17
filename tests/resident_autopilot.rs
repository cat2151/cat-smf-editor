//! Drives the real editor and player like a person: start the editor (it plays
//! the restored song), press t, then keys in the patch browser.
//!
//! Keys are posted to the windows, so nothing else on the desktop receives them.
//! The editor runs with its own LOCALAPPDATA and TEMP under the system temp
//! directory, so the user's session and trace files stay untouched.
#![cfg(windows)]

#[path = "resident_autopilot/support.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use support::*;

/// About a person's pace on ↓.
const PRESS_INTERVAL: Duration = Duration::from_millis(900);
const PRESSES: usize = 5;

/// The patch under the cursor at the end must be the one the editor plays.
#[test]
#[ignore = "starts the real editor and player: windows open and audio plays"]
fn t_then_down_keys_play_the_last_chosen_patch() {
    let work = std::env::temp_dir().join(format!("cat-autopilot-{}", std::process::id()));
    let (local, temp, songs) = (work.join("local"), work.join("temp"), work.join("songs"));
    for dir in [&local.join("cat-smf-editor"), &temp, &songs] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(
        songs.join("two-tracks.mid"),
        cat_smf_editor::smf_io::encode(&song()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        local.join("cat-smf-editor/status.json"),
        serde_json::json!({
            "navigation": { "last_mid": songs.join("two-tracks.mid") },
            "session": { "file": "two-tracks.mid", "track": 2, "mbt": null, "tick": null,
                         "row": null, "note_mode": true },
            "program_target": "Auto",
        })
        .to_string(),
    )
    .unwrap();
    let logs = temp.join("cat-resident-log");
    let mut run = Run {
        editor: Command::new(env!("CARGO_BIN_EXE_cat-smf-editor"))
            .env("LOCALAPPDATA", &local)
            .env("TEMP", &temp)
            .env("TMP", &temp)
            .spawn()
            .unwrap(),
        player: None,
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let editor = wait_for("the editor window", Duration::from_secs(30), || {
            window(|process, title| {
                process == run.editor.id() && title.starts_with("cat-smf-editor")
            })
        });
        // The restored song starts playing; its plugins open on the first frames.
        std::thread::sleep(Duration::from_secs(3));
        press(editor.0, T);
        let player = wait_for("the player process", Duration::from_secs(30), || {
            child_of(run.editor.id())
        });
        run.player = Some(player);
        let browser = wait_for("the patch browser", Duration::from_secs(30), || {
            window(|process, title| process == player && title == "Browse patches")
        });
        wait_for(
            "the catalog and an idle player",
            Duration::from_secs(90),
            || {
                let state = last_state(&logs)?;
                (field(&state, "results=", " status=")? != "0" && idle(&state)).then_some(())
            },
        );
        std::thread::sleep(Duration::from_secs(1));
        let mut pressed = Instant::now();
        for _ in 0..PRESSES {
            press(browser.0, DOWN);
            pressed = Instant::now();
            std::thread::sleep(PRESS_INTERVAL);
        }
        let settled = wait_until(Duration::from_secs(20), || {
            last_state(&logs).is_some_and(|state| idle(&state))
        });
        let state = last_state(&logs).unwrap_or_default();
        (settled.then(|| pressed.elapsed()), state)
    }));
    run.close();
    println!("{}", timeline(&logs));
    // Kept on failure for the traces.
    println!("work directory: {}", work.display());
    let (settled, state) = match result {
        Ok(outcome) => outcome,
        Err(panic) => std::panic::resume_unwind(panic),
    };
    let chosen = field(&state, "selected=", " desired=").and_then(|t| t.split_once(' '));
    let playing = field(&state, "applied=", " offered=");
    assert!(
        settled.is_some(),
        "the player never became idle after the last ↓: {state}"
    );
    assert_eq!(
        chosen.map(|(_, display)| display),
        playing,
        "the patch under the cursor is not the one playing (settled {settled:?} after the last ↓)"
    );
    let _ = std::fs::remove_dir_all(&work);
}

/// Esc after the chosen patch has reached the editor: the browser hides, and
/// the editor keeps playing that patch and saves it into the song.
#[test]
#[ignore = "starts the real editor and player: windows open and audio plays"]
fn escape_after_a_patch_applies_keeps_it() {
    let work = std::env::temp_dir().join(format!("cat-autopilot-esc-{}", std::process::id()));
    let logs = work.join("temp/cat-resident-log");
    let mut run = launch(&work);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let browser = open_patch_browser(&mut run, &logs);
        for _ in 0..2 {
            press(browser, DOWN);
            std::thread::sleep(PRESS_INTERVAL);
        }
        assert!(
            wait_until(Duration::from_secs(20), || last_state(&logs)
                .is_some_and(|state| idle(&state))),
            "the player never became idle after ↓"
        );
        let before = last_state(&logs).unwrap();
        press(browser, ESCAPE);
        let hidden = wait_until(Duration::from_secs(10), || {
            last_state(&logs)
                .is_some_and(|state| field(&state, "state shown=", " patch_open=") == Some("None"))
        });
        // Anything Esc could still send would reach the editor within this.
        std::thread::sleep(Duration::from_secs(2));
        let visible =
            window(|process, title| Some(process) == run.player && title == "Browse patches")
                .is_some();
        (
            hidden,
            visible,
            before,
            last_state(&logs).unwrap_or_default(),
        )
    }));
    run.close();
    println!("{}", timeline(&logs));
    println!("work directory: {}", work.display());
    let (hidden, visible, before, after) = match result {
        Ok(outcome) => outcome,
        Err(panic) => std::panic::resume_unwind(panic),
    };
    assert!(hidden && !visible, "Esc did not hide the browser: {after}");
    let applied = field(&before, "applied=", " offered=").unwrap();
    assert_ne!(applied, "-", "no patch reached the editor before Esc");
    assert_eq!(field(&after, "applied=", " offered="), Some(applied));
    let editor = lines(&logs.join("editor.log"));
    let player = lines(&logs.join("player.log"));
    let hid = player
        .iter()
        .find(|l| l.ends_with(" player hide browser"))
        .unwrap();
    let hid_at = hid.split_once(' ').unwrap().0.parse::<u128>().unwrap();
    let late: Vec<_> = editor
        .iter()
        .filter(|l| l.contains(" editor selected ") || l.contains(" editor apply "))
        .filter(|l| l.split_once(' ').unwrap().0.parse::<u128>().unwrap() > hid_at)
        .collect();
    assert!(late.is_empty(), "the editor applied after Esc: {late:?}");
    let saved = saved_presets(&work.join("songs"));
    assert_eq!(
        saved,
        vec![applied.to_owned(); 2],
        "the saved song's patches"
    );
    let _ = std::fs::remove_dir_all(&work);
}

/// Delays between the last ↓ and Esc, each run this many times.
const ESCAPE_DELAYS: [u64; 8] = [0, 40, 70, 100, 120, 300, 1000, 2000];
const ESCAPE_RUNS: usize = 3;

/// ↓ then Esc after each delay: a patch the editor already played before Esc
/// must be the one it keeps. Prints one row per run.
#[test]
#[ignore = "starts the real editor and player: windows open and audio plays"]
fn escape_soon_after_down_keeps_a_patch_that_played() {
    let mut rows = Vec::new();
    for delay in ESCAPE_DELAYS {
        for run in 0..ESCAPE_RUNS {
            let row = escape_after(delay, run);
            println!("{row:?}");
            rows.push(row);
        }
    }
    println!("delay | phase at Esc | played at | Esc at | answers after Esc | kept");
    for row in &rows {
        println!(
            "{} | {} | {:?} | {:?} | {} | {}",
            row.delay,
            row.phase,
            row.played_at,
            row.hid_at,
            row.late_answers,
            row.kept()
        );
    }
    let discarded: Vec<_> = rows
        .iter()
        .filter(|row| row.played_before_escape() && row.kept() != "chosen")
        .collect();
    assert!(
        discarded.is_empty(),
        "a patch the editor played before Esc was dropped: {discarded:#?}"
    );
}

/// One ↓-then-Esc run. Times are milliseconds after the last ↓ was posted.
#[derive(Debug)]
struct Escaped {
    delay: u64,
    previous: String,
    chosen: Option<String>,
    /// Where the chosen patch was in the player's pipeline when Esc hid the browser.
    phase: &'static str,
    /// The editor's `answer Ok` for the chosen patch.
    played_at: Option<i128>,
    /// The player's `hide browser`.
    hid_at: Option<i128>,
    late_answers: usize,
    saved: Vec<String>,
}

impl Escaped {
    fn played_before_escape(&self) -> bool {
        matches!((self.played_at, self.hid_at), (Some(p), Some(h)) if p <= h)
    }

    fn kept(&self) -> &'static str {
        let all = |name: &str| self.saved.iter().all(|s| s == name);
        match &self.chosen {
            Some(chosen) if all(chosen) => "chosen",
            _ if all(&self.previous) => "previous",
            _ => "other",
        }
    }
}

fn escape_after(delay: u64, run: usize) -> Escaped {
    let work = std::env::temp_dir().join(format!(
        "cat-autopilot-esc{delay}-{run}-{}",
        std::process::id()
    ));
    let logs = work.join("temp/cat-resident-log");
    let mut launched = launch(&work);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let browser = open_patch_browser(&mut launched, &logs);
        press(browser, DOWN);
        let previous = wait_for("the first patch to apply", Duration::from_secs(20), || {
            let state = last_state(&logs).filter(|state| idle(state))?;
            let applied = field(&state, "applied=", " offered=")?;
            (applied != "-").then(|| applied.to_owned())
        });
        std::thread::sleep(Duration::from_millis(500));
        let pressed = now_millis();
        press(browser, DOWN);
        std::thread::sleep(Duration::from_millis(delay));
        press(browser, ESCAPE);
        let settled = wait_until(Duration::from_secs(20), || {
            last_state(&logs).is_some_and(|state| {
                idle(&state) && field(&state, "state shown=", " patch_open=") == Some("None")
            })
        });
        assert!(settled, "the player never settled after Esc");
        // Anything Esc could still send would reach the editor within this.
        std::thread::sleep(Duration::from_secs(2));
        (previous, pressed)
    }));
    launched.close();
    println!("{}", timeline(&logs));
    println!("work directory: {}", work.display());
    let (previous, pressed) = match result {
        Ok(outcome) => outcome,
        Err(panic) => std::panic::resume_unwind(panic),
    };
    let after = |millis: u128| millis as i128 - pressed as i128;
    let player = stamped(&logs.join("player.log"));
    let editor = stamped(&logs.join("editor.log"));
    let chosen = player
        .iter()
        .filter(|(at, _)| *at >= pressed)
        .filter_map(|(_, line)| field(line, "selected=", " desired="))
        .filter_map(|ticket| ticket.split_once(' ').map(|(_, display)| display))
        .find(|display| *display != previous)
        .map(str::to_owned);
    let hid_line = player
        .iter()
        .position(|(at, line)| *at >= pressed && line.ends_with(" player hide browser"));
    let before_escape = player[..hid_line.unwrap_or(player.len())]
        .iter()
        .rev()
        .find(|(_, line)| line.contains(" player state "))
        .map_or("", |(_, line)| line.as_str());
    let phase = chosen.as_deref().map_or("unselected", |chosen| {
        let holds =
            |start, end| field(before_escape, start, end).is_some_and(|t| t.ends_with(chosen));
        if holds("applied=", " offered=") {
            "applied"
        } else if holds("applying=", " applied=") {
            "applying"
        } else if holds("preparing=", " applying=") {
            "preparing"
        } else {
            "selected"
        }
    });
    let played_at = chosen.as_ref().and_then(|chosen| {
        let quoted = format!("{chosen:?}");
        let from = editor
            .iter()
            .position(|(_, line)| line.contains(" editor selected ") && line.contains(&quoted))?;
        editor[from..]
            .iter()
            .find(|(_, line)| line.contains(" editor answer Ok"))
            .map(|(at, _)| *at)
    });
    let late_answers = editor
        .iter()
        .filter(|(at, line)| {
            hid_line.is_some_and(|i| *at > player[i].0) && line.contains(" editor answer ")
        })
        .count();
    let saved = saved_presets(&work.join("songs"));
    let _ = std::fs::remove_dir_all(&work);
    Escaped {
        delay,
        previous,
        chosen,
        phase,
        played_at: played_at.map(after),
        hid_at: hid_line.map(|i| after(player[i].0)),
        late_answers,
        saved,
    }
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// A trace's lines with their milliseconds since the Unix epoch.
fn stamped(path: &Path) -> Vec<(u128, String)> {
    lines(path)
        .into_iter()
        .filter_map(|line| Some((line.split_once(' ')?.0.parse().ok()?, line)))
        .collect()
}

/// Starts the editor on the two-track song with its own LOCALAPPDATA and TEMP.
fn launch(work: &Path) -> Run {
    let (local, temp, songs) = (work.join("local"), work.join("temp"), work.join("songs"));
    for dir in [&local.join("cat-smf-editor"), &temp, &songs] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(
        songs.join("two-tracks.mid"),
        cat_smf_editor::smf_io::encode(&song()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        local.join("cat-smf-editor/status.json"),
        serde_json::json!({
            "navigation": { "last_mid": songs.join("two-tracks.mid") },
            "session": { "file": "two-tracks.mid", "track": 2, "mbt": null, "tick": null,
                         "row": null, "note_mode": true },
            "program_target": "Auto",
        })
        .to_string(),
    )
    .unwrap();
    Run {
        editor: Command::new(env!("CARGO_BIN_EXE_cat-smf-editor"))
            .env("LOCALAPPDATA", &local)
            .env("TEMP", &temp)
            .env("TMP", &temp)
            .spawn()
            .unwrap(),
        player: None,
    }
}

/// Presses t and waits until the browser lists patches and nothing is in flight.
fn open_patch_browser(run: &mut Run, logs: &Path) -> usize {
    let editor = wait_for("the editor window", Duration::from_secs(30), || {
        window(|process, title| process == run.editor.id() && title.starts_with("cat-smf-editor"))
    });
    // The restored song starts playing; its plugins open on the first frames.
    std::thread::sleep(Duration::from_secs(3));
    press(editor.0, T);
    let player = wait_for("the player process", Duration::from_secs(30), || {
        child_of(run.editor.id())
    });
    run.player = Some(player);
    let browser = wait_for("the patch browser", Duration::from_secs(30), || {
        window(|process, title| process == player && title == "Browse patches")
    });
    wait_for(
        "the catalog and an idle player",
        Duration::from_secs(90),
        || {
            let state = last_state(logs)?;
            (field(&state, "results=", " status=")? != "0" && idle(&state)).then_some(())
        },
    );
    std::thread::sleep(Duration::from_secs(1));
    browser.0
}

/// The instrument preset of each note track in the song the editor saved.
fn saved_presets(directory: &Path) -> Vec<String> {
    let songs: Vec<PathBuf> = std::fs::read_dir(directory)
        .unwrap()
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|e| e == "mid"))
        .collect();
    assert_eq!(songs.len(), 1, "saved songs: {songs:?}");
    let music = cat_smf_editor::smf_io::load(&songs[0]).unwrap();
    music.tracks[1..]
        .iter()
        .map(|track| {
            let settings = cat_smf_editor::instrument_layers::parse(track).unwrap();
            settings
                .layers
                .into_iter()
                .next()
                .and_then(|l| l.patch)
                .map(|p| p.display)
                .unwrap_or_default()
        })
        .collect()
}
