//! Windows, keys, processes, and the resident traces for the autopilot tests.
use cat_smf_editor::model::{Event, Music, Track};
use std::ffi::c_void;
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

#[link(name = "user32")]
unsafe extern "system" {
    fn EnumWindows(
        callback: unsafe extern "system" fn(*mut c_void, isize) -> i32,
        data: isize,
    ) -> i32;
    fn GetWindowThreadProcessId(window: *mut c_void, process: *mut u32) -> u32;
    fn IsWindowVisible(window: *mut c_void) -> i32;
    fn GetWindowTextW(window: *mut c_void, text: *mut u16, len: i32) -> i32;
    fn PostMessageW(window: *mut c_void, message: u32, w: usize, l: isize) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, process: u32) -> *mut c_void;
    fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32;
    fn TerminateProcess(handle: *mut c_void, code: u32) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, process: u32) -> *mut c_void;
    fn Process32FirstW(snapshot: *mut c_void, entry: *mut ProcessEntry) -> i32;
    fn Process32NextW(snapshot: *mut c_void, entry: *mut ProcessEntry) -> i32;
}

#[repr(C)]
struct ProcessEntry {
    size: u32,
    usage: u32,
    process: u32,
    heap: usize,
    module: u32,
    threads: u32,
    parent: u32,
    priority: i32,
    flags: u32,
    exe: [u16; 260],
}

const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_CLOSE: u32 = 0x0010;
pub const T: (usize, isize, bool) = (0x54, 0x14, false);
pub const DOWN: (usize, isize, bool) = (0x28, 0x50, true);
pub const ESCAPE: (usize, isize, bool) = (0x1b, 0x01, false);

/// A conductor and two note tracks sharing one program, so t changes both (bulk).
pub fn song() -> Music {
    let event = |tick, status, meta_type, data: &[u8]| Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    };
    const EIGHTH: u64 = 240;
    const NOTES: u64 = 960;
    let part = |channel: u8, pitches: &[u8]| {
        let mut events = vec![event(0, 0xc0 | channel, 0, &[0])];
        for i in 0..NOTES {
            let pitch = pitches[i as usize % pitches.len()];
            events.push(event(i * EIGHTH, 0x90 | channel, 0, &[pitch, 90]));
            events.push(event(
                i * EIGHTH + EIGHTH - 20,
                0x80 | channel,
                0,
                &[pitch, 0],
            ));
        }
        events.push(event(NOTES * EIGHTH, 0xff, 0x2f, &[]));
        Track { events }
    };
    Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![
                    event(0, 0xff, 0x51, &[0x07, 0xa1, 0x20]),
                    event(NOTES * EIGHTH, 0xff, 0x2f, &[]),
                ],
            },
            part(0, &[60, 64, 67, 72, 67, 64]),
            part(1, &[36, 43]),
        ],
    }
}

/// Closes the editor, which ends the player; anything left is terminated.
pub struct Run {
    pub editor: Child,
    pub player: Option<u32>,
}

impl Run {
    pub fn close(&mut self) {
        if let Some((editor, _)) = window(|process, title| {
            process == self.editor.id() && title.starts_with("cat-smf-editor")
        }) {
            unsafe { PostMessageW(editor as *mut c_void, WM_CLOSE, 0, 0) };
        }
        let closed = wait_until(Duration::from_secs(15), || {
            if let Some((editor, _)) = window(|process, title| {
                process == self.editor.id() && title.starts_with("cat-smf-editor")
            }) {
                press(editor, (0x59, 0x15, false)); // y: save and exit
            }
            self.editor.try_wait().ok().flatten().is_some()
        });
        if !closed {
            let _ = self.editor.kill();
            let _ = self.editor.wait();
        }
        if let Some(player) = self.player.take() {
            end(player, Duration::from_secs(15));
        }
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn end(process: u32, timeout: Duration) {
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const PROCESS_TERMINATE: u32 = 0x0001;
    let handle = unsafe { OpenProcess(SYNCHRONIZE | PROCESS_TERMINATE, 0, process) };
    if handle.is_null() {
        return;
    }
    unsafe {
        if WaitForSingleObject(handle, timeout.as_millis() as u32) != 0 {
            eprintln!("player {process} did not exit; terminating");
            TerminateProcess(handle, 1);
        }
        CloseHandle(handle);
    }
}

pub fn wait_until(timeout: Duration, mut done: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while !done() {
        if start.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    true
}

pub fn wait_for<T>(what: &str, timeout: Duration, mut probe: impl FnMut() -> Option<T>) -> T {
    let mut found = None;
    let done = wait_until(timeout, || {
        found = probe();
        found.is_some()
    });
    assert!(done, "timed out waiting for {what}");
    found.unwrap()
}

/// The editor's own player, never another one the user may be running.
pub fn child_of(parent: u32) -> Option<u32> {
    const TH32CS_SNAPPROCESS: u32 = 0x2;
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot as isize == -1 {
        return None;
    }
    let mut entry = ProcessEntry {
        size: std::mem::size_of::<ProcessEntry>() as u32,
        usage: 0,
        process: 0,
        heap: 0,
        module: 0,
        threads: 0,
        parent: 0,
        priority: 0,
        flags: 0,
        exe: [0; 260],
    };
    let mut found = None;
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    while more {
        let len = entry.exe.iter().position(|&c| c == 0).unwrap_or(260);
        if entry.parent == parent
            && String::from_utf16_lossy(&entry.exe[..len]) == "cat-plugin-player.exe"
        {
            found = Some(entry.process);
            break;
        }
        more = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snapshot) };
    found
}

pub fn window(wanted: impl Fn(u32, &str) -> bool) -> Option<(usize, u32)> {
    unsafe extern "system" fn collect(window: *mut c_void, data: isize) -> i32 {
        let found = unsafe { &mut *(data as *mut Vec<(usize, u32, String)>) };
        let mut process = 0;
        unsafe { GetWindowThreadProcessId(window, &mut process) };
        if unsafe { IsWindowVisible(window) } != 0 {
            let mut text = [0u16; 256];
            let len = unsafe { GetWindowTextW(window, text.as_mut_ptr(), 256) };
            let title = String::from_utf16_lossy(&text[..len.max(0) as usize]);
            found.push((window as usize, process, title));
        }
        1
    }
    let mut found = Vec::<(usize, u32, String)>::new();
    unsafe { EnumWindows(collect, &mut found as *mut _ as isize) };
    found
        .into_iter()
        .find(|(_, process, title)| wanted(*process, title))
        .map(|(window, process, _)| (window, process))
}

/// Arrow keys are extended keys.
pub fn press(window: usize, (key, scan, extended): (usize, isize, bool)) {
    let flags = 1 | (scan << 16) | if extended { 1 << 24 } else { 0 };
    unsafe {
        PostMessageW(window as *mut c_void, WM_KEYDOWN, key, flags);
        PostMessageW(
            window as *mut c_void,
            WM_KEYUP,
            key,
            flags | 0xc000_0000u32 as isize,
        );
    }
}

pub fn lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The player's latest selection pipeline line.
pub fn last_state(logs: &Path) -> Option<String> {
    lines(&logs.join("player.log"))
        .into_iter()
        .rev()
        .find(|line| line.contains(" player state "))
}

/// Nothing chosen is waiting for preparation, the editor, or its answer.
pub fn idle(state: &str) -> bool {
    [
        ("desired=", " confirm="),
        ("preparing=", " applying="),
        ("applying=", " applied="),
        ("offered=", " busy["),
    ]
    .iter()
    .all(|(start, end)| field(state, start, end) == Some("-"))
}

pub fn field<'a>(line: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let rest = &line[line.find(start)? + start.len()..];
    Some(&rest[..rest.find(end)?])
}

/// Both traces merged by time, in seconds since the first line.
pub fn timeline(logs: &Path) -> String {
    let mut all: Vec<(u128, String)> = ["editor.log", "player.log"]
        .iter()
        .flat_map(|name| lines(&logs.join(name)))
        .filter_map(|line| {
            let (millis, rest) = line.split_once(' ')?;
            Some((millis.parse().ok()?, rest.to_owned()))
        })
        .collect();
    all.sort_by_key(|(millis, _)| *millis);
    let first = all.first().map_or(0, |(millis, _)| *millis);
    all.iter()
        .map(|(millis, rest)| {
            let at = millis - first;
            format!("{:>4}.{:03} {rest}\n", at / 1000, at % 1000)
        })
        .collect()
}
