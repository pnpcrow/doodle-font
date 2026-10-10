use super::*;
use std::net::TcpListener;
fn folder() -> PathBuf {
    let path = std::env::temp_dir().join(format!("doodle-install-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    path
}
fn asset(url: String, data: &[u8]) -> Asset {
    Asset {
        name: "model.gguf".into(),
        url,
        sha256: format!("{:x}", Sha256::digest(data)),
        bytes: data.len() as u64,
    }
}
fn run_fetch(root: &Path, asset: &Asset) -> Result<(), String> {
    let (tx, _rx) = std::sync::mpsc::channel();
    let cancel = AtomicBool::new(false);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fetch(
            &reqwest::Client::new(),
            asset,
            root,
            &cancel,
            &tx,
            0,
            asset.bytes,
        ))
}
#[test]
fn verified_cache_does_not_need_network() {
    let root = folder();
    let a = asset("http://127.0.0.1:1".into(), b"verified");
    fs::write(root.join(&a.name), b"verified").unwrap();
    run_fetch(&root, &a).unwrap();
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn resumes_partial_or_restarts_when_range_is_ignored() {
    for supports_range in [true, false] {
        let root = folder();
        fs::write(root.join("model.gguf.partial"), b"abc").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let n = socket.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..n])
                .to_lowercase()
                .contains("range: bytes=3-"));
            let response = if supports_range {
                "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 3-5/6\r\nContent-Length: 3\r\nConnection: close\r\n\r\ndef"
            } else {
                "HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabcdef"
            };
            socket.write_all(response.as_bytes()).unwrap();
        });
        run_fetch(&root, &asset(url, b"abcdef")).unwrap();
        server.join().unwrap();
        assert_eq!(fs::read(root.join("model.gguf")).unwrap(), b"abcdef");
        assert!(!root.join("model.gguf.partial").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn checksum_failure_preserves_previous_install() {
    let root = folder();
    fs::write(root.join("model.gguf"), b"old!").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for _ in 0..3 {
            let (mut socket, _) = listener.accept().unwrap();
            let mut b = [0; 1024];
            let read = socket.read(&mut b).unwrap();
            assert!(read > 0);
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nbad!")
                .unwrap();
        }
    });
    assert!(run_fetch(&root, &asset(url, b"good")).is_err());
    server.join().unwrap();
    assert_eq!(fs::read(root.join("model.gguf")).unwrap(), b"old!");
    assert_eq!(
        fs::metadata(root.join("model.gguf.partial")).unwrap().len(),
        0
    );
    assert!(!root.join("installed.json").exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn truncated_download_is_retried_from_saved_offset() {
    let root = folder();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for step in 0..2 {
            let (mut socket, _) = listener.accept().unwrap();
            let mut b = [0; 2048];
            let n = socket.read(&mut b).unwrap();
            if step == 1 {
                assert!(String::from_utf8_lossy(&b[..n])
                    .to_lowercase()
                    .contains("range: bytes=3-"));
            }
            socket.write_all(if step==0{b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabc".as_slice()}else{b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 3-5/6\r\nContent-Length: 3\r\nConnection: close\r\n\r\ndef".as_slice()}).unwrap();
        }
    });
    run_fetch(&root, &asset(url, b"abcdef")).unwrap();
    server.join().unwrap();
    assert_eq!(fs::read(root.join("model.gguf")).unwrap(), b"abcdef");
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn rejects_zip_paths_outside_install_folder() {
    let root = folder();
    let zip_path = root.join("runtime.zip");
    let mut zip = zip::ZipWriter::new(File::create(&zip_path).unwrap());
    zip.start_file("../outside.exe", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"bad").unwrap();
    zip.finish().unwrap();
    assert!(extract(&zip_path, &root.join("stage"), &AtomicBool::new(false)).is_err());
    assert!(!root.join("outside.exe").exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn cancellation_preserves_download_for_next_attempt() {
    let root = folder();
    fs::write(root.join("model.gguf.partial"), b"abc").unwrap();
    let (tx, _) = std::sync::mpsc::channel();
    let a = asset("http://127.0.0.1:1".into(), b"abcdef");
    let cancel = AtomicBool::new(true);
    let r = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fetch(
            &reqwest::Client::new(),
            &a,
            &root,
            &cancel,
            &tx,
            0,
            6,
        ));
    assert!(r.unwrap_err().contains("취소"));
    assert_eq!(fs::read(root.join("model.gguf.partial")).unwrap(), b"abc");
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn missing_report_checks_runtime_dependencies_and_model_sizes() {
    let root = folder();
    fs::create_dir(root.join("runtime")).unwrap();
    fs::write(root.join("runtime/llama-server.exe"), b"stub").unwrap();
    fs::write(root.join("assistant.gguf"), b"short").unwrap();
    let missing = missing(&root);
    assert!(missing.contains(&"assistant.gguf".into()));
    assert!(!missing.iter().any(|s| s.contains("embedding")));
    assert!(missing.contains(&"runtime/llama-server-impl.dll".into()));
    assert!(!missing.contains(&"runtime/llama-server.exe".into()));
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn decision_install_is_independent_of_gemma_and_legacy_embeddings() {
    let root = folder();
    let package = package_manifest(Package::Needle);
    for asset in &package.files {
        let file = File::create(root.join(&asset.name)).unwrap();
        file.set_len(asset.bytes).unwrap();
    }
    assert_eq!(
        package_missing(&root, Package::Needle),
        vec!["installed.json"]
    );
    fs::write(root.join("installed.json"), b"{}").unwrap();
    assert!(package_missing(&root, Package::Needle).is_empty());
    assert!(!missing(&root).is_empty());
    assert_eq!(manifest().files.len(), 3);
    assert!(!manifest()
        .files
        .iter()
        .any(|a| a.name.contains("embedding")));
    fs::remove_dir_all(root).unwrap();
}
