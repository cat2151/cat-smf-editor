//! engine に繋ぐ。応答する engine があればそれを使い、無ければ設定の `run.exe` を起動する。
use super::client::Client;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// VOICEVOX 本体と同じ port。本体の engine が起動中ならそれを使う。
pub const PORT: u16 = 50021;
pub const START_TIMEOUT: Duration = Duration::from_secs(60);

/// 繋がった engine。自分で起動した process だけを持ち、drop で落とす。
pub struct Engine {
    pub client: Client,
    child: Option<Child>,
}

impl Engine {
    /// この engine の process を自分で起動したなら、その PID。
    pub fn started_pid(&self) -> Option<u32> {
        self.child.as_ref().map(Child::id)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// 応答する engine が無く、起動できる `run.exe` も設定されていない。
    NoExecutable,
    Failed(String),
}

/// 設定された `run.exe` が実在すれば、そのパス。
pub fn executable(configured: Option<&Path>) -> Option<&Path> {
    configured.filter(|path| path.is_file())
}

pub fn connect(port: u16, configured: Option<&Path>, timeout: Duration) -> Result<Engine, Failure> {
    let client = Client::new(&format!("http://127.0.0.1:{port}"));
    if client.version().is_ok() {
        return Ok(Engine {
            client,
            child: None,
        });
    }
    let path = executable(configured).ok_or(Failure::NoExecutable)?;
    let child = spawn(path, port)
        .map_err(|e| Failure::Failed(format!("Cannot start {}: {e}", path.display())))?;
    let mut engine = Engine {
        client,
        child: Some(child),
    };
    let deadline = Instant::now() + timeout;
    while engine.client.version().is_err() {
        if let Some(Ok(Some(status))) = engine.child.as_mut().map(Child::try_wait) {
            return Err(Failure::Failed(format!(
                "{} exited before answering ({status})",
                path.display()
            )));
        }
        if Instant::now() >= deadline {
            return Err(Failure::Failed(format!(
                "{} did not answer on port {port} in {} s",
                path.display(),
                timeout.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Ok(engine)
}

fn spawn(path: &Path, port: u16) -> std::io::Result<Child> {
    let mut command = Command::new(path);
    command
        .args(["--port", &port.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.spawn()
}
