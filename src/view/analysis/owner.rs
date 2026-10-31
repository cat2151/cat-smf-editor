//! An owned window stays above its owner and never covers other applications;
//! egui viewports cannot be given an owner, so it is set on the native window.
use std::ffi::c_void;
use std::ptr::{null, null_mut};

type Hwnd = *mut c_void;

const GWLP_HWNDPARENT: i32 = -8;
const SWP_NOSIZE: u32 = 0x0001;
const SWP_NOMOVE: u32 = 0x0002;
const SWP_NOACTIVATE: u32 = 0x0010;

#[link(name = "user32")]
unsafe extern "system" {
    fn GetForegroundWindow() -> Hwnd;
    fn GetAncestor(window: Hwnd, flags: u32) -> Hwnd;
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

/// Makes the editor, which must be the foreground window, own our window titled
/// `title`; false while either is not found.
pub fn adopt(title: &str) -> bool {
    // SAFETY: a null or foreign handle is rejected before use.
    let editor = unsafe { GetForegroundWindow() };
    !editor.is_null() && ours(editor) && adopt_by(editor as isize, title)
}

/// The window that owns our foreground window, or that window itself; it outlives the
/// windows it owns.
pub fn editor() -> Option<isize> {
    const GA_ROOTOWNER: u32 = 3;
    // SAFETY: the handles come from the window manager and are only passed back to it.
    unsafe {
        let window = GetForegroundWindow();
        (!window.is_null() && ours(window)).then(|| GetAncestor(window, GA_ROOTOWNER) as isize)
    }
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
