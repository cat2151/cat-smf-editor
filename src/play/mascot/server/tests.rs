use super::*;

#[test]
#[cfg(windows)]
fn finds_neighbor_release_and_path_without_using_working_directory() {
    let paths = candidates(
        Path::new("X:/projects/cat-smf-editor/target/debug/cat-smf-editor.exe"),
        Some(std::ffi::OsStr::new("X:/tools")),
    );
    assert_eq!(
        paths[0],
        PathBuf::from("X:/projects/cat-smf-editor/target/debug/mascot-render-server.exe")
    );
    assert_eq!(
        paths[1],
        PathBuf::from("X:/projects/mascot-render-server/target/release/mascot-render-server.exe")
    );
    assert_eq!(paths[2], PathBuf::from("X:/tools/mascot-render-server.exe"));
}

#[test]
fn healthy_external_server_is_never_launched_or_owned() {
    let mut server = Server::default();
    server
        .ensure_started(|| true, || panic!("must reuse external server"))
        .unwrap();
    assert!(server.child.is_none());
}

#[test]
fn failed_launch_is_throttled() {
    let mut server = Server::default();
    assert!(server
        .ensure_started(|| false, || Err("missing executable".into()))
        .is_err());
    server
        .ensure_started(|| false, || panic!("must not retry each playback tick"))
        .unwrap();
    server.last_start = Some(Instant::now() - Duration::from_secs(6));
    assert!(server
        .ensure_started(|| false, || Err("retry".into()))
        .is_err());
}

#[cfg(windows)]
fn process_exists(pid: u32) -> bool {
    use std::os::windows::process::CommandExt;
    Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 0 }} else {{ exit 1 }}")])
        .creation_flags(0x0800_0000)
        .status().unwrap().success()
}

#[test]
#[cfg(windows)]
fn owned_process_is_not_duplicated_and_is_reaped_on_drop() {
    use std::os::windows::process::CommandExt;
    let mut server = Server::default();
    server
        .ensure_started(
            || false,
            || {
                Command::new("powershell.exe")
                    .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 60"])
                    .creation_flags(0x0800_0000)
                    .spawn()
                    .map_err(|e| e.to_string())
            },
        )
        .unwrap();
    let pid = server.child.as_ref().unwrap().id();
    assert!(process_exists(pid));
    server
        .ensure_started(|| false, || panic!("startup must not duplicate process"))
        .unwrap();
    drop(server);
    assert!(!process_exists(pid));
}

#[test]
#[ignore = "launches the installed mascot window; run explicitly with server stopped"]
#[cfg(windows)]
fn real_server_starts_answers_playback_and_stops() {
    assert!(
        mascot_render_client::mascot_render_server_healthcheck().is_err(),
        "stop external server before this test"
    );
    let mut server = Server::default();
    let mut request = crate::play::mascot::stopped();
    request.playing = true;
    let _ = server.sync(&request);
    let pid = server.child.as_ref().expect("server launched").id();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        request.sent_unix_ms = mascot_render_protocol::now_unix_ms();
        if server.sync(&request).is_ok() {
            break;
        }
        assert!(Instant::now() < deadline, "server did not become ready");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(server.child.as_ref().unwrap().id(), pid);
    server.sync(&crate::play::mascot::stopped()).unwrap();
    drop(server);
    assert!(!process_exists(pid));
    assert!(mascot_render_client::mascot_render_server_healthcheck().is_err());
}
