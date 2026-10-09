//! Local, on-demand native inference. No models or user content leave this machine.
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
    async fn start(embedding: bool, cancel: &AtomicBool) -> Result<Self, String> {
        if !installed() {
            return Err("AI 연결 화면에서 '로컬 AI 설치'를 눌러 주세요.".into());
        }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let key = uuid::Uuid::new_v4().to_string();
        let stem = if embedding { "embedding" } else { "assistant" };
        let mut command = Command::new(executable());
        command
            .arg("-m")
            .arg(root().join(format!("{stem}.gguf")))
            .arg("--mmproj")
            .arg(root().join(format!("{stem}-vision.gguf")))
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
        if embedding {
            command.args([
                "--embedding",
                "--pooling",
                "mean",
                "-b",
                "8192",
                "-ub",
                "8192",
            ]);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        drop(listener);
        let mut server = Self {
            child: command
                .spawn()
                .map_err(|e| format!("로컬 AI 실행 실패: {e}"))?,
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
                return Err("AI 작업을 취소했어요.".into());
            }
            if server
                .child
                .try_wait()
                .map_err(|e| e.to_string())?
                .is_some()
            {
                return Err(
                    "로컬 AI 실행이 중단되었어요. 모델 파일과 메모리를 확인해 주세요.".into(),
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
        Err("로컬 모델 준비 시간이 초과되었어요.".into())
    }
}

/// Cancellation drops the active HTTP future and both owned server processes.
pub fn run(
    mut payload: Value,
    cancel: Arc<AtomicBool>,
    status: Sender<String>,
) -> Result<Value, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async {
        let work = async {
            crate::ai::validate_target(&payload, payload["action"] != "assist")?;
            if payload["config"]["provider"] == "local" {
                let _ = status.send("EmbeddingGemma 2 준비 중".into());
                let embedding = Server::start(true, &cancel).await?;
                let _ = status.send("프롬프트와 그림을 함께 분석 중".into());
                let glyph: crate::model::Glyph = serde_json::from_value(payload["glyph"].clone()).map_err(|e|e.to_string())?;
                let picture = crate::ai::canvas_image(&glyph.artwork)?;
                let mut input = vec![json!({"content":[{"type":"text","text":format!("task: sentence similarity | query: {}",payload["prompt"].as_str().unwrap_or(""))},{"type":"image_url","image_url":{"url":picture}}]})];
                let references: Vec<crate::model::Template>=payload["references"].as_array().into_iter().flatten().take(8).filter_map(|v|serde_json::from_value(v.clone()).ok()).collect();
                let references: Vec<_>=references.iter().filter(|t|t.permissions.ai && t.permissions.display && t.original.is_some()).collect();
                for template in &references {
                    input.push(json!({"content":[{"type":"image_url","image_url":{"url":crate::ai::canvas_image(template.original.as_ref().unwrap())?}}]}));
                }
                let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(180)).build().map_err(|e|e.to_string())?;
                let response: Value = client.post(format!("{}/embeddings", embedding.endpoint)).bearer_auth(&embedding.key).json(&json!({"input":input,"encoding_format":"float"})).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.json().await.map_err(|e|e.to_string())?;
                let vector = response["data"][0]["embedding"].as_array().ok_or("멀티모달 임베딩 응답이 없어요.")?;
                if vector.len()!=768 || vector.iter().any(|v|v.as_f64().is_none_or(|n|!n.is_finite())) { return Err("EmbeddingGemma 2 분석 결과가 올바르지 않아요.".into()); }
                // The vector is a semantic fingerprint, not a generated explanation.
                let embedding_value = json!(vector);
                let mut similar: Vec<(String,f64)>=references.iter().enumerate().filter_map(|(i,t)|{
                    let other=response["data"][i+1]["embedding"].as_array()?;
                    if other.len()!=vector.len(){return None;}
                    let score=vector.iter().zip(other).map(|(a,b)|a.as_f64().unwrap_or(0.)*b.as_f64().unwrap_or(0.)).sum();
                    Some((t.name.clone(),score))
                }).collect();
                similar.sort_by(|a,b|b.1.total_cmp(&a.1));
                let matches=similar.iter().take(3).map(|(n,s)|format!("{n} ({s:.2})")).collect::<Vec<_>>().join(" · ");
                drop(embedding);
                let _ = status.send("Gemma 4 E2B 어시스턴트 준비 중".into());
                let assistant = Server::start(false, &cancel).await?;
                payload["config"] = json!({"provider":"openai","endpoint":assistant.endpoint,"model":"local","key":assistant.key,"local":true});
                payload["sendReference"] = json!(true);
                let _ = status.send(if payload["action"] == "generate" {
                    "그림 후보를 그리는 중"
                } else {
                    "그림을 확인하고 답변 작성 중"
                }.into());
                let mut result = perform(payload).await?;
                result["embedding"] = embedding_value;
                result["matches"] = json!(matches);
                Ok(result)
            } else {
                let _ = status.send("AI가 그림을 확인하고 작업 중".into());
                perform(payload).await
            }
        };
        tokio::pin!(work);
        loop { tokio::select! {
            result = &mut work => return result,
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                if cancel.load(Ordering::Relaxed) { return Err("AI 작업을 취소했어요.".into()); }
            }
        }}
    })
}
async fn perform(payload: Value) -> Result<Value, String> {
    if payload["action"] == "assist" {
        crate::ai::assist(payload)
            .await
            .map(|s| json!({"answer":s}))
    } else {
        crate::ai::generate(payload)
            .await
            .map(|a| json!({"artwork":a}))
    }
}
