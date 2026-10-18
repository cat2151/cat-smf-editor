//! Scan without opening an audio device or creating plugin instances.
use cat_smf_editor::host::{Host, PluginInfo};
use std::{
    ffi::{c_char, c_void},
    sync::mpsc,
};

pub struct Scan {
    // Drop the host (and its callbacks) before their context.
    host: Host,
    sender: Box<mpsc::Sender<Result<(), String>>>,
    receiver: mpsc::Receiver<Result<(), String>>,
}

unsafe extern "C" fn wake(_: *mut c_void) {}
unsafe extern "C" fn done(user: *mut c_void, error: *const c_char) {
    let sender = unsafe { &*(user as *const mpsc::Sender<Result<(), String>>) };
    let result = unsafe { cat_smf_editor::host::error_string(error) }.map_or(Ok(()), Err);
    let _ = sender.send(result);
}

impl Scan {
    pub fn start(rescan: bool) -> Result<Self, String> {
        let host = unsafe { Host::new(wake, std::ptr::null_mut()) }
            .ok_or("Cannot create plugin scanner")?;
        let (tx, receiver) = mpsc::channel();
        let mut scan = Self {
            host,
            sender: Box::new(tx),
            receiver,
        };
        let user = (&mut *scan.sender) as *mut _ as *mut c_void;
        // SAFETY: boxed sender remains alive until after host destruction.
        if !unsafe { scan.host.scan_async(rescan, done, user) } {
            return Err("Cannot start plugin scan".into());
        }
        Ok(scan)
    }

    pub fn poll(&self) -> Option<Result<Vec<PluginInfo>, String>> {
        self.host.pump();
        self.receiver
            .try_recv()
            .ok()
            .map(|result| result.map(|()| self.host.plugins()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "scans installed plugins; no audio device or plugin instances"]
    fn real_scan_completes_without_opening_an_audio_device() {
        let directory = tempfile::tempdir().unwrap();
        let _library = cat_smf_editor::native_library::load_at(directory.path()).unwrap();
        let scan = Scan::start(false).unwrap();
        let start = std::time::Instant::now();
        loop {
            if let Some(result) = scan.poll() {
                let plugins = result.unwrap();
                eprintln!("Detected {} plugins", plugins.len());
                break;
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(120));
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
