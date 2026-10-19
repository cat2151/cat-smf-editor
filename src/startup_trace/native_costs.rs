//! Opt-in phase measurements without opening a device or sending notes.
use crate::host::{self, Host};
use std::{
    ffi::{c_char, c_void},
    sync::mpsc,
    time::{Duration, Instant},
};

unsafe extern "C" fn wake(_: *mut c_void) {}
unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
    let sender = unsafe { Box::from_raw(user as *mut mpsc::Sender<Result<i32, String>>) };
    let result = unsafe { host::error_string(error) }.map_or(Ok(id), Err);
    let _ = sender.send(result);
}

#[test]
#[ignore = "opens actual default instruments; no device or notes; keeps instances alive together"]
fn native_instance_creation_phase_costs() {
    let _library =
        crate::native_library::load_at(&crate::storage_paths::cache_directory().unwrap()).unwrap();
    let key = crate::chain::default_instrument().unwrap();
    let mut live = Vec::new();
    for index in 0..5 {
        let begin = Instant::now();
        let host = unsafe { Host::new(wake, std::ptr::null_mut()) }.unwrap();
        let host_time = begin.elapsed();
        let begin = Instant::now();
        let plugin = host.restore_plugin(&key).unwrap();
        let metadata_time = begin.elapsed();
        let (sender, receiver) = mpsc::channel::<Result<i32, String>>();
        let begin = Instant::now();
        unsafe {
            host.create_instance(
                plugin.index,
                48000,
                1024,
                created,
                Box::into_raw(Box::new(sender)) as *mut c_void,
            );
        }
        let request_time = begin.elapsed();
        let id = loop {
            host.pump();
            if let Ok(result) = receiver.try_recv() {
                break result.unwrap();
            }
            assert!(begin.elapsed() < Duration::from_secs(20));
            std::thread::sleep(Duration::from_millis(1));
        };
        let instance_time = begin.elapsed();
        let begin = Instant::now();
        let processor = host.create_chain(id, &[], false, 48000, 1024).unwrap();
        let graph_time = begin.elapsed();
        let begin = Instant::now();
        host.settle_processor(&processor, 48000).unwrap();
        println!("instance {index}: host={host_time:?} metadata={metadata_time:?} request={request_time:?} instance_total={instance_time:?} graph={graph_time:?} settle={:?}", begin.elapsed());
        live.push((processor, host)); // Processor drops before its host.
    }
}
