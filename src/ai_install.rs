//! Native, resumable local model installation. All large files are streamed.
#[cfg(test)]
#[path = "ai_install_tests.rs"]
mod tests;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Deserialize, Serialize)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
}
#[derive(Deserialize, Serialize)]
pub struct Manifest {
    pub version: u32,
    pub files: Vec<Asset>,
}
pub fn manifest() -> Manifest {
    serde_json::from_str(include_str!("../assets/local-ai-manifest.json"))
        .expect("embedded model manifest")
}
#[derive(Clone, Debug)]
pub struct Progress {
    pub message: String,
    pub fraction: f32,
}
pub fn missing(root: &Path) -> Vec<String> {
    let mut out: Vec<_> = manifest()
        .files
        .into_iter()
        .filter(|a| a.name != "runtime.zip")
        .filter(|a| fs::metadata(root.join(&a.name)).map_or(true, |m| m.len() != a.bytes))
        .map(|a| a.name)
        .collect();
    let exe = super::local_ai::executable_at(root);
    let runtime = exe.parent().unwrap_or(root);
    for name in [
        "llama-server.exe",
        "llama-server-impl.dll",
        "llama.dll",
        "llama-common.dll",
        "mtmd.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu-x64.dll",
    ] {
        if !runtime.join(name).is_file() {
            out.push(format!("runtime/{name}"));
        }
    }
    if !root.join("installed.json").is_file() {
        out.push("installed.json".into());
    }
    out
}
fn check_cancel(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("설치를 취소했어요. 다시 설치하면 내려받던 파일을 이어받습니다.".into())
    } else {
        Ok(())
    }
}
fn verified(path: &Path, asset: &Asset, cancel: &AtomicBool) -> Result<bool, String> {
    if fs::metadata(path).map_or(true, |m| m.len() != asset.bytes) {
        return Ok(false);
    }
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        check_cancel(cancel)?;
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()) == asset.sha256)
}
fn progress(tx: &Sender<Progress>, message: String, done: u64, total: u64) {
    let _ = tx.send(Progress {
        message,
        fraction: (done as f64 / total.max(1) as f64).clamp(0., 1.) as f32,
    });
}
async fn fetch(
    client: &reqwest::Client,
    asset: &Asset,
    root: &Path,
    cancel: &AtomicBool,
    tx: &Sender<Progress>,
    done: u64,
    total: u64,
) -> Result<(), String> {
    check_cancel(cancel)?;
    let path = root.join(&asset.name);
    progress(
        tx,
        format!("{} · 기존 파일 검증 중", asset.name),
        done,
        total,
    );
    if verified(&path, asset, cancel)? {
        return Ok(());
    }
    let partial = root.join(format!("{}.partial", asset.name));
    for attempt in 0..3 {
        check_cancel(cancel)?;
        let result = async {
            let mut offset = fs::metadata(&partial).map_or(0, |m| m.len());
            if offset > asset.bytes {
                File::create(&partial).map_err(|e| e.to_string())?;
                offset = 0;
            }
            if offset < asset.bytes {
                progress(
                    tx,
                    format!(
                        "{} · 연결 중{}",
                        asset.name,
                        if offset > 0 { " (이어받기)" } else { "" }
                    ),
                    done + offset,
                    total,
                );
                let mut request = client.get(&asset.url);
                if offset > 0 {
                    request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
                }
                let mut response = request
                    .send()
                    .await
                    .map_err(|e| format!("연결 실패: {}", e.without_url()))?;
                if !response.status().is_success() {
                    return Err(format!("서버 응답 {}", response.status().as_u16()));
                }
                if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
                    let expected = format!("bytes {offset}-");
                    if !response
                        .headers()
                        .get(reqwest::header::CONTENT_RANGE)
                        .and_then(|v| v.to_str().ok())
                        .is_some_and(|s| {
                            s.starts_with(&expected) && s.ends_with(&format!("/{}", asset.bytes))
                        })
                    {
                        return Err("서버의 이어받기 범위가 올바르지 않아요.".into());
                    }
                } else {
                    offset = 0;
                }
                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(offset == 0)
                    .append(offset > 0)
                    .open(&partial)
                    .map_err(|e| format!("저장 공간·권한을 확인해 주세요: {e}"))?;
                let mut last = Instant::now() - Duration::from_secs(1);
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|_| "다운로드 연결이 끊겼어요.")?
                {
                    check_cancel(cancel)?;
                    if offset + chunk.len() as u64 > asset.bytes {
                        return Err("다운로드 크기가 예상보다 커요.".into());
                    }
                    file.write_all(&chunk)
                        .map_err(|e| format!("저장 공간·권한을 확인해 주세요: {e}"))?;
                    offset += chunk.len() as u64;
                    if last.elapsed() > Duration::from_millis(200) {
                        progress(
                            tx,
                            format!(
                                "{} · {:.0} / {:.0} MB",
                                asset.name,
                                offset as f64 / 1e6,
                                asset.bytes as f64 / 1e6
                            ),
                            done + offset,
                            total,
                        );
                        last = Instant::now();
                    }
                }
                file.sync_all().map_err(|e| e.to_string())?;
                if offset != asset.bytes {
                    return Err("다운로드가 중간에 끊겼어요.".into());
                }
            }
            progress(
                tx,
                format!("{} · SHA256 검증 중", asset.name),
                done + asset.bytes,
                total,
            );
            if !verified(&partial, asset, cancel)? {
                File::create(&partial).map_err(|e| e.to_string())?;
                return Err("파일 검증에 실패했어요. 새로 내려받습니다.".into());
            }
            fs::rename(&partial, &path).map_err(|e| {
                format!("파일을 교체하지 못했어요. 실행 중인 다른 AI 작업을 종료해 주세요: {e}")
            })?;
            Ok(())
        }
        .await;
        match result {
            Ok(()) => return Ok(()),
            Err(e) => {
                check_cancel(cancel)?;
                if attempt == 2 {
                    return Err(format!("{}: {e}", asset.name));
                }
                progress(
                    tx,
                    format!("{} · 재시도 {}/3: {e}", asset.name, attempt + 2),
                    done,
                    total,
                );
            }
        }
    }
    unreachable!()
}
fn extract(zip_path: &Path, destination: &Path, cancel: &AtomicBool) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(File::open(zip_path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if archive.len() > 512 {
        return Err("실행기 압축 파일에 항목이 너무 많아요.".into());
    }
    let mut total = 0u64;
    for i in 0..archive.len() {
        check_cancel(cancel)?;
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry
            .enclosed_name()
            .ok_or("실행기 압축 경로가 올바르지 않아요.")?;
        total = total.saturating_add(entry.size());
        if total > 400_000_000 || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("실행기 압축 내용이 올바르지 않아요.".into());
        }
        let path = destination.join(name);
        if entry.is_dir() {
            fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut file = File::create(path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn clean_owned_dir(root: &Path, child: &Path) {
    if let (Ok(root), Ok(child)) = (root.canonicalize(), child.canonicalize()) {
        if child.parent() == Some(root.as_path()) {
            let _ = fs::remove_dir_all(child);
        }
    }
}
fn install_runtime(root: &Path, cancel: &AtomicBool) -> Result<(), String> {
    let stage = root.join(format!("runtime-install-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        extract(&root.join("runtime.zip"), &stage, cancel)?;
        check_cancel(cancel)?;
        if !stage.join("llama-server.exe").is_file() {
            return Err("실행기가 압축 파일에 없어요.".into());
        }
        let runtime = root.join("runtime");
        let backup = root.join(format!("runtime-backup-{}", uuid::Uuid::new_v4()));
        let had_runtime = runtime.exists();
        if had_runtime {
            fs::rename(&runtime, &backup).map_err(|e| {
                format!(
                    "실행기를 교체하지 못했어요. 다른 AI 작업을 종료하고 다시 시도해 주세요: {e}"
                )
            })?;
        }
        if let Err(e) = fs::rename(&stage, &runtime) {
            if had_runtime {
                let _ = fs::rename(&backup, &runtime);
            }
            return Err(e.to_string());
        }
        if had_runtime {
            clean_owned_dir(root, &backup);
        }
        Ok(())
    })();
    clean_owned_dir(root, &stage);
    result
}
pub fn install(root: PathBuf, cancel: Arc<AtomicBool>, tx: Sender<Progress>) -> Result<(), String> {
    fs::create_dir_all(&root).map_err(|e| format!("설치 폴더를 만들지 못했어요: {e}"))?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("install.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock()
        .map_err(|_| "다른 창에서 로컬 AI를 설치 중이에요. 완료 후 다시 확인해 주세요.")?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async{
        let client=reqwest::Client::builder().connect_timeout(Duration::from_secs(30)).read_timeout(Duration::from_secs(60)).redirect(reqwest::redirect::Policy::limited(10)).build().map_err(|e|e.to_string())?;
        let manifest=manifest();let total=manifest.files.iter().map(|f|f.bytes).sum();let mut done=0;
        for asset in &manifest.files{
            let future=fetch(&client,asset,&root,&cancel,&tx,done,total);tokio::pin!(future);
            loop{tokio::select!{result=&mut future=>{result?;break;},_=tokio::time::sleep(Duration::from_millis(100))=>{check_cancel(&cancel)?;}}}
            done+=asset.bytes;
        }
        progress(&tx,"실행기 압축 해제 중".into(),done,total);
        install_runtime(&root,&cancel)?;check_cancel(&cancel)?;
        // License texts are compiled into the app, so repair also works offline.
        fs::write(root.join("LICENSE-llama.cpp.txt"),include_str!("../assets/ai-notices/LICENSE-llama.cpp.txt")).map_err(|e|e.to_string())?;
        fs::write(root.join("LICENSE-Apache-2.0.txt"),include_str!("../assets/ai-notices/LICENSE-Apache-2.0.txt")).map_err(|e|e.to_string())?;
        let pending=root.join("installed.json.pending");
        fs::write(&pending,serde_json::to_vec_pretty(&manifest).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        fs::rename(pending,root.join("installed.json")).map_err(|e|e.to_string())?;
        let missing=missing(&root);if !missing.is_empty(){return Err(format!("설치 후 확인이 필요해요: {}",missing.join(", ")));}
        progress(&tx,"설치 완료 · 로컬 AI를 사용할 수 있어요".into(),total,total);Ok(())
    })
}
