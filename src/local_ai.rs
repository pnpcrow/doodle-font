//! Local, on-demand native inference. No models or user content leave this machine.
use crate::i18n::{format as tf, text as tr};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    time::Duration,
};

pub fn root() -> PathBuf {
    std::env::var_os("DOODLE_AI_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                std::env::var_os("LOCALAPPDATA")
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| {
                        PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default())
                            .join("AppData/Local")
                            .into_os_string()
                    }),
            )
            .join("DoodleFont/ai")
        })
}
fn executable() -> PathBuf {
    executable_at(&root())
}
pub(crate) fn executable_at(root: &std::path::Path) -> PathBuf {
    let dir = root.join("runtime");
    if dir.join("llama-server.exe").exists() {
        return dir.join("llama-server.exe");
    }
    std::fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path().join("llama-server.exe"))
        .find(|p| p.is_file())
        .unwrap_or_else(|| dir.join("llama-server.exe"))
}
pub fn installed() -> bool {
    crate::ai_install::missing(&root()).is_empty()
}
struct Server {
    child: Child,
    endpoint: String,
    key: String,
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Server {
    async fn start(cancel: &AtomicBool) -> Result<Self, String> {
        if !installed() {
            return Err(tr("설정에서 로컬 AI를 선택하면 설치할 수 있어요.").into());
        }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let key = uuid::Uuid::new_v4().to_string();
        let mut command = Command::new(executable());
        command
            .arg("-m")
            .arg(root().join("assistant.gguf"))
            .arg("--mmproj")
            .arg(root().join("assistant-vision.gguf"))
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--api-key",
                &key,
                "-c",
                "8192",
                "-np",
                "1",
                "-ngl",
                "0",
                "--no-mmproj-offload",
                "-t",
                "6",
                "--no-webui",
                "--log-disable",
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
        let mut server = Self {
            child: command
                .spawn()
                .map_err(|e| tf("로컬 AI 실행 실패: {0}", &[format!("{}", e)]))?,
            endpoint: format!("http://127.0.0.1:{port}/v1"),
            key,
        };
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| e.to_string())?;
        for _ in 0..600 {
            if cancel.load(Ordering::Relaxed) {
                return Err(tr("AI 작업을 취소했어요.").into());
            }
            if server
                .child
                .try_wait()
                .map_err(|e| e.to_string())?
                .is_some()
            {
                return Err(
                    tr("로컬 AI 실행이 중단되었어요. 모델 파일과 메모리를 확인해 주세요.").into(),
                );
            }
            if client
                .get(server.endpoint.replace("/v1", "/health"))
                .bearer_auth(&server.key)
                .send()
                .await
                .is_ok_and(|r| r.status().is_success())
            {
                return Ok(server);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        Err(tr("로컬 모델 준비 시간이 초과되었어요.").into())
    }
}

/// Cancellation drops the active HTTP future and owned Gemma process.
pub fn run(
    mut payload: Value,
    cancel: Arc<AtomicBool>,
    status: Sender<crate::ai_progress::Progress>,
) -> Result<Value, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async {
        let work = async {
            crate::ai::validate_target(&payload, payload["action"] != "assist")?;
            if payload["config"]["provider"] == "local" {
                let _ = status.send(tr("Gemma 4 E2B 어시스턴트 준비 중").into());
                let assistant = Server::start(&cancel).await?;
                payload["config"] = json!({"provider":"openai","endpoint":assistant.endpoint,"model":"local","key":assistant.key,"local":true});
                payload["sendReference"] = json!(true);
                let _ = status.send(if payload["action"] == "generate" {
                    tr("그림 후보를 그리는 중")
                } else {
                    tr("그림을 확인하고 답변 작성 중")
                }.into());
                perform(payload).await
            } else {
                let _ = status.send(tr("AI가 그림을 확인하고 작업 중").into());
                perform(payload).await
            }
        };
        tokio::pin!(work);
        loop { tokio::select! {
            result = &mut work => return result,
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                if cancel.load(Ordering::Relaxed) { return Err(tr("AI 작업을 취소했어요.").into()); }
            }
        }}
    })
}
async fn perform(payload: Value) -> Result<Value, String> {
    if payload["action"] == "assist" {
        crate::ai::assist(payload)
            .await
            .map(|s| json!({"answer":s}))
    } else if payload["config"]["local"] == true
        && serde_json::from_value::<crate::model::Glyph>(payload["glyph"].clone())
            .is_ok_and(|g| crate::typography::is_text(&g))
    {
        crate::ai::generate_typography(payload).await
    } else {
        crate::ai::generate(payload)
            .await
            .map(|a| json!({"artwork":a}))
    }
}
