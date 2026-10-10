//! Optional text-only decision model. No API keys or images are sent to Needle.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Off,
    Needle,
}
#[derive(Default, Serialize, Deserialize)]
pub struct Options {
    #[serde(default)]
    pub decision: Mode,
}
impl Options {
    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let pending = path.with_extension("pending");
        std::fs::write(
            &pending,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(pending, path).map_err(|e| e.to_string())
    }
}
pub fn root() -> PathBuf {
    std::env::var_os("DOODLE_NEEDLE_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::local_ai::root().join("decision"))
}
pub fn installed() -> bool {
    crate::ai_install::package_missing(&root(), crate::ai_install::Package::Needle).is_empty()
}
struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
pub fn infer(prompt: &str, cancel: Arc<AtomicBool>) -> Result<Value, String> {
    infer_at(&root(), prompt, cancel, Duration::from_secs(12))
}
pub fn infer_at(
    root: &std::path::Path,
    prompt: &str,
    cancel: Arc<AtomicBool>,
    timeout: Duration,
) -> Result<Value, String> {
    if prompt.len() > 8000 {
        return Err("판단할 입력이 너무 길어요.".into());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("AI 작업을 취소했어요.".into());
    }
    if !crate::ai_install::package_missing(root, crate::ai_install::Package::Needle).is_empty() {
        return Err("판단 모델이 설치되지 않았어요.".into());
    }
    // Unique fixed-schema file prevents two application windows from sharing mutable state.
    let tools_path = root.join(format!("tools-{}.json", uuid::Uuid::new_v4()));
    std::fs::write(&tools_path, include_bytes!("../assets/needle-tools.json"))
        .map_err(|e| e.to_string())?;
    let result = (|| {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let mut command = Command::new(root.join("needle.exe"));
        command
            .arg("--model")
            .arg(root.join("needle3.cact"))
            .arg("--tools")
            .arg(&tools_path)
            .args([
                "--serve",
                "--port",
                &port.to_string(),
                "--max",
                "192",
                "--threads",
                "2",
                "--fail-input-overflow",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        drop(listener);
        let mut server = Server(
            command
                .spawn()
                .map_err(|_| "판단 모델을 실행하지 못했어요.")?,
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        runtime.block_on(async {
            let work = async {
                let client = reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none())
                    .timeout(timeout).build().map_err(|e| e.to_string())?;
                let endpoint = format!("http://127.0.0.1:{port}");
                loop {
                    if server.0.try_wait().map_err(|e| e.to_string())?.is_some() { return Err("판단 모델이 종료되었어요.".into()); }
                    if client.post(format!("{endpoint}/reset")).json(&json!({})).send().await.is_ok_and(|r| r.status().is_success()) { break; }
                    tokio::time::sleep(Duration::from_millis(30)).await;
                }
                // UTF-8 HTTP avoids the Windows runner's narrow-character argv conversion.
                let mut response = client.post(format!("{endpoint}/complete")).json(&json!({"input":prompt})).send().await
                    .map_err(|_| "판단 모델에 연결하지 못했어요.")?.error_for_status().map_err(|_| "판단 모델이 오류를 반환했어요.")?;
                let mut bytes = Vec::new();
                while let Some(chunk) = response.chunk().await.map_err(|_| "판단 응답이 끊겼어요.")? {
                    if bytes.len() + chunk.len() > 64 * 1024 { return Err("판단 응답이 너무 커요.".into()); }
                    bytes.extend_from_slice(&chunk);
                }
                serde_json::from_slice(&bytes).map_err(|_| "판단 응답 형식이 맞지 않아요.".into())
            };
            tokio::pin!(work);
            let deadline = tokio::time::sleep(timeout);
            tokio::pin!(deadline);
            loop { tokio::select! {
                r = &mut work => return r,
                _ = &mut deadline => return Err("판단 시간이 초과되었어요.".into()),
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    if cancel.load(Ordering::Relaxed) { return Err("AI 작업을 취소했어요.".into()); }
                }
            }}
        })
    })();
    let _ = std::fs::remove_file(tools_path);
    result
}
