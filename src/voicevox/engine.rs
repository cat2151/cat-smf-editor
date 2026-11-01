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
        .map_err(|e| Failure::Failed(format!("{}を起動できません: {e}", path.display())))?;
    let mut engine = Engine {
        client,
        child: Some(child),
    };
    let deadline = Instant::now() + timeout;
    while engine.client.version().is_err() {
        if let Some(Ok(Some(status))) = engine.child.as_mut().map(Child::try_wait) {
            return Err(Failure::Failed(format!(
                "{}が応答する前に終了しました ({status})",
                path.display()
            )));
        }
        if Instant::now() >= deadline {
            return Err(Failure::Failed(format!(
                "{}がport {port}で{} s以内に応答しませんでした",
                path.display(),
                timeout.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Ok(engine)
}

fn spawn(path: &Path, port: u16) -> std::io::Result<Child> {
    let mut command = hidden(path);
    command
        .args(["--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn()
}

/// `port` で待ち受ける engine を、誰が起動したかに関わらず落とす。
pub fn kill_listener(port: u16) -> Result<(), String> {
    let output = hidden("netstat")
        .args(["-ano", "-p", "TCP"])
        .output()
        .map_err(|e| format!("netstatを実行できません: {e}"))?;
    for pid in listeners(&String::from_utf8_lossy(&output.stdout), port) {
        let output = hidden("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output()
            .map_err(|e| format!("taskkillを実行できません: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "VOICEVOX engine (PID {pid})を終了できません: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }
    Ok(())
}

/// `netstat -ano` の出力から、`port` を LISTEN している PID。
fn listeners(netstat: &str, port: u16) -> Vec<u32> {
    let suffix = format!(":{port}");
    let mut pids = Vec::new();
    for line in netstat.lines() {
        let columns: Vec<&str> = line.split_whitespace().collect();
        // 状態の列は locale で訳されうるので、相手が未定（`:0`）であることで LISTEN を見分ける。
        if let ["TCP", local, foreign, _, pid] = columns[..]
            && local.ends_with(&suffix)
            && foreign.ends_with(":0")
            && let Ok(pid) = pid.parse::<u32>()
            && pid != 0
            && !pids.contains(&pid)
        {
            pids.push(pid);
        }
    }
    pids
}

fn hidden(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    command.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::listeners;

    #[test]
    fn listeners_are_the_pids_listening_on_the_port() {
        let netstat = "
  TCP    127.0.0.1:50021        0.0.0.0:0              LISTENING       14916
  TCP    127.0.0.1:56025        127.0.0.1:50021        TIME_WAIT       0
  TCP    127.0.0.1:56030        127.0.0.1:50021        ESTABLISHED     777
  TCP    127.0.0.1:500210       0.0.0.0:0              LISTENING       888
  TCP    [::1]:50021            [::]:0                 LISTENING       14916
";
        assert_eq!(listeners(netstat, 50021), vec![14916]);
    }
}
