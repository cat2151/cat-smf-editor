//! Resident mode has only browser windows; inspect visibility without changing its IPC.
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Activity {
    shown: bool,
    refreshed: Option<Instant>,
}

impl Activity {
    pub fn poll(&mut self, process: u32) -> bool {
        let mut query = Query {
            process,
            visible: false,
            title: None,
        };
        unsafe { EnumWindows(find_visible, &mut query as *mut Query as isize) };
        self.observe(query.visible, Instant::now())
    }

    fn observe(&mut self, shown: bool, now: Instant) -> bool {
        let refresh = (shown
            && self
                .refreshed
                .is_none_or(|last| now.saturating_duration_since(last) >= Duration::from_secs(60)))
            || self.shown != shown;
        self.shown = shown;
        if refresh {
            self.refreshed = Some(now);
        }
        refresh
    }
}

struct Query {
    process: u32,
    visible: bool,
    title: Option<&'static str>,
}

pub(super) fn visible(process: u32, title: &'static str) -> bool {
    let mut query = Query {
        process,
        visible: false,
        title: Some(title),
    };
    unsafe { EnumWindows(find_visible, &mut query as *mut Query as isize) };
    query.visible
}

unsafe extern "system" fn find_visible(window: *mut std::ffi::c_void, data: isize) -> i32 {
    let query = unsafe { &mut *(data as *mut Query) };
    let mut process = 0;
    unsafe { GetWindowThreadProcessId(window, &mut process) };
    if process == query.process && unsafe { IsWindowVisible(window) } != 0 {
        if let Some(title) = query.title {
            let mut text = [0u16; 256];
            let len = unsafe { GetWindowTextW(window, text.as_mut_ptr(), 256) };
            if String::from_utf16_lossy(&text[..len.max(0) as usize]) != title {
                return 1;
            }
        }
        query.visible = true;
        return 0;
    }
    1
}

#[link(name = "user32")]
unsafe extern "system" {
    fn EnumWindows(
        callback: unsafe extern "system" fn(*mut std::ffi::c_void, isize) -> i32,
        data: isize,
    ) -> i32;
    fn IsWindowVisible(window: *mut std::ffi::c_void) -> i32;
    fn GetWindowTextW(window: *mut std::ffi::c_void, text: *mut u16, len: i32) -> i32;
    fn GetWindowThreadProcessId(window: *mut std::ffi::c_void, process: *mut u32) -> u32;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn open_browser_refreshes_every_minute_and_closing_starts_a_full_idle_period() {
        let mut activity = Activity::default();
        let start = Instant::now();
        assert!(!activity.observe(false, start));
        assert!(activity.observe(true, start));
        assert!(!activity.observe(true, start + Duration::from_secs(59)));
        assert!(activity.observe(true, start + Duration::from_secs(60)));
        assert!(activity.observe(true, start + Duration::from_secs(600)));
        assert!(activity.observe(false, start + Duration::from_secs(610)));
        assert!(!activity.observe(false, start + Duration::from_secs(900)));
        assert!(activity.observe(true, start + Duration::from_secs(910)));
    }
}
