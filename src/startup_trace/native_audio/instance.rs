//! Offline-only host with state capture for data-home comparison.
use crate::host::{self, Host, PluginKey, Processor};
use std::{
    ffi::{c_char, c_void},
    sync::mpsc,
    time::{Duration, Instant},
};

unsafe extern "C" fn wake(_: *mut c_void) {}
unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
    let sender = unsafe { Box::from_raw(user as *mut mpsc::Sender<Result<i32, String>>) };
    let result = unsafe { host::error_string(error) }.map_or_else(
        || {
            if id >= 0 {
                Ok(id)
            } else {
                Err("Negative instance ID".into())
            }
        },
        Err,
    );
    let _ = sender.send(result);
}

pub(super) struct Instance {
    processor: Option<Processor>,
    host: Host,
    id: i32,
}

impl Instance {
    pub(super) fn open(key: &PluginKey, state: Option<&[u8]>, effect: bool) -> Self {
        let host = unsafe { Host::new(wake, std::ptr::null_mut()) }.unwrap();
        let plugin = host.restore_plugin(key).unwrap();
        let (sender, receiver) = mpsc::channel::<Result<i32, String>>();
        unsafe {
            host.create_instance(
                plugin.index,
                48000,
                1024,
                created,
                Box::into_raw(Box::new(sender)) as *mut c_void,
            );
        }
        let begin = Instant::now();
        let id = loop {
            host.pump();
            if let Ok(result) = receiver.try_recv() {
                break result.unwrap();
            }
            assert!(begin.elapsed() < Duration::from_secs(20));
            std::thread::sleep(Duration::from_millis(1));
        };
        if let Some(state) = state {
            host.load_state(id, state).unwrap();
        }
        let processor = if effect {
            host.create_chain(-1, &[id], false, 48000, 1024)
        } else {
            host.create_chain(id, &[], false, 48000, 1024)
        }
        .unwrap();
        host.settle_processor(&processor, 48000).unwrap();
        Self {
            processor: Some(processor),
            host,
            id,
        }
    }

    pub(super) fn save(&self) -> Vec<u8> {
        self.host.save_state(self.id).unwrap()
    }
    pub(super) fn audio(&self) -> host::ProcessorRef {
        unsafe { self.processor.as_ref().unwrap().audio_ref() }
    }
    pub(super) fn pump(&self) {
        self.host.pump();
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.processor.take();
        self.host.destroy_instance(self.id);
    }
}
