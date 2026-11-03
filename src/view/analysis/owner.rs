//! An owned window stays above its owner and never covers other applications;
//! egui viewports cannot be given an owner, so it is set on the native window.
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicIsize, Ordering};

type Hwnd = *mut c_void;

const GWLP_HWNDPARENT: i32 = -8;
const SWP_NOSIZE: u32 = 0x0001;
const SWP_NOMOVE: u32 = 0x0002;
const SWP_NOACTIVATE: u32 = 0x0010;

/// The root viewport's native window; 0 until the first frame.
static EDITOR: AtomicIsize = AtomicIsize::new(0);

#[link(name = "user32")]
unsafe extern "system" {
    fn FindWindowExW(parent: Hwnd, after: Hwnd, class: *const u16, title: *const u16) -> Hwnd;
    fn GetWindowThreadProcessId(window: Hwnd, process: *mut u32) -> u32;
    fn SetWindowLongPtrW(window: Hwnd, index: i32, value: isize) -> isize;
    fn SetWindowPos(window: Hwnd, after: Hwnd, x: i32, y: i32, cx: i32, cy: i32, flags: u32)
        -> i32;
}

fn ours(window: Hwnd) -> bool {
    let mut process = 0;
    // SAFETY: a stale handle only yields no process id.
    unsafe { GetWindowThreadProcessId(window, &mut process) };
    process == std::process::id()
}

/// Records the root viewport's window. Windows destroys owned windows with their
/// owner, so only this window, which lives as long as the app, may own others.
pub fn remember(frame: &eframe::Frame) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    if let Ok(handle) = frame.window_handle()
        && let RawWindowHandle::Win32(window) = handle.as_raw()
    {
        EDITOR.store(window.hwnd.get(), Ordering::Relaxed);
    }
}

/// Makes the editor own our window titled `title`; false while either is not found.
pub fn adopt(title: &str) -> bool {
    editor().is_some_and(|editor| adopt_by(editor, title))
}

/// The root viewport's window, once [`remember`]ed.
pub fn editor() -> Option<isize> {
    Some(EDITOR.load(Ordering::Relaxed)).filter(|&window| window != 0)
}

/// Makes `editor` own our window titled `title`; false while it is not found.
pub fn adopt_by(editor: isize, title: &str) -> bool {
    let editor = editor as Hwnd;
    // SAFETY: the handles come from the window manager and are only passed back to it.
    unsafe {
        let title: Vec<u16> = title.encode_utf16().chain([0]).collect();
        let mut window = null_mut();
        loop {
            window = FindWindowExW(null_mut(), window, null(), title.as_ptr());
            if window.is_null() {
                return false;
            }
            if window != editor && ours(window) {
                SetWindowLongPtrW(window, GWLP_HWNDPARENT, editor as isize);
                // A new owner does not reorder; the window restored at startup is
                // still under the editor that was shown after it.
                let flags = SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE;
                SetWindowPos(window, null_mut(), 0, 0, 0, 0, flags);
                return true;
            }
        }
    }
}
