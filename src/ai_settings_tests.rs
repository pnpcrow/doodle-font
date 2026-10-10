use super::*;

struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("doodle-connections-test-{}", uuid::Uuid::new_v4())))
    }
    fn path(&self) -> std::path::PathBuf {
        self.0.join("ai-connections.dat")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn windows_encryption_survives_reload_without_plaintext_files() {
    let fixture = Fixture::new();
    let mut connections = Connections::load(&fixture.path()).unwrap();
    assert_eq!(connections.provider, "off");
    connections.remember(
        "openai",
        "https://example.invalid/v1",
        "테스트 모델",
        "test-key-unique-123456",
    );
    connections.save(&fixture.path()).unwrap();
    drop(connections);
    let restored = Connections::load(&fixture.path()).unwrap();
    assert_eq!(restored.provider, "openai");
    assert_eq!(restored.profile("openai").model, "테스트 모델");
    assert_eq!(
        restored.key("openai", "https://example.invalid/v1/"),
        "test-key-unique-123456"
    );
    let bytes = std::fs::read(fixture.path()).unwrap();
    for secret in [b"test-key-unique-123456".as_slice(), b"example.invalid"] {
        assert!(!bytes.windows(secret.len()).any(|part| part == secret));
    }
    assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), 1);
}

#[test]
fn keys_are_scoped_to_provider_and_endpoint_and_can_be_removed() {
    let fixture = Fixture::new();
    let mut connections = Connections::default();
    connections.remember("openai", "https://one.invalid/v1", "model-a", "key-a");
    connections.remember("openai", "https://two.invalid/v1", "model-b", "key-b");
    assert!(connections
        .key("anthropic", "https://one.invalid/v1")
        .is_empty());
    assert!(connections
        .key("openai", "https://one.invalid/other")
        .is_empty());
    connections.remember("anthropic", "https://one.invalid/v1", "model-c", "key-c");
    connections.remember("local", "", "", "");
    connections.save(&fixture.path()).unwrap();
    let mut restored = Connections::load(&fixture.path()).unwrap();
    assert_eq!(restored.provider, "local");
    assert_eq!(
        restored.profile("openai").endpoint,
        "https://two.invalid/v1"
    );
    assert_eq!(restored.key("openai", "https://one.invalid/v1"), "key-a");
    restored.remember("openai", "https://two.invalid/v1", "model-b", "");
    restored.save(&fixture.path()).unwrap();
    let restored = Connections::load(&fixture.path()).unwrap();
    assert!(restored.key("openai", "https://two.invalid/v1").is_empty());
    assert_eq!(restored.key("anthropic", "https://one.invalid/v1"), "key-c");
}

#[test]
fn disabled_selection_survives_restart_and_preserves_saved_connections() {
    let fixture = Fixture::new();
    let mut connections = Connections::default();
    connections.remember(
        "openai",
        "https://example.invalid/v1",
        "saved-model",
        "saved-key",
    );
    connections.remember("off", "", "", "");
    connections.save(&fixture.path()).unwrap();
    let restored = Connections::load(&fixture.path()).unwrap();
    assert_eq!(restored.provider, "off");
    assert_eq!(restored.profile("openai").model, "saved-model");
    assert_eq!(
        restored.key("openai", "https://example.invalid/v1"),
        "saved-key"
    );
}

#[test]
fn corrupt_settings_are_preserved_and_never_report_credentials() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(&fixture.0).unwrap();
    let bad = b"plaintext-test-key-do-not-print";
    std::fs::write(fixture.path(), bad).unwrap();
    let error = Connections::load(&fixture.path()).err().unwrap();
    assert!(!error.contains("plaintext-test-key"));
    assert_eq!(std::fs::read(fixture.path()).unwrap(), bad);
    let mut damaged = HEADER.to_vec();
    damaged.extend_from_slice(bad);
    std::fs::write(fixture.path(), &damaged).unwrap();
    assert!(Connections::load(&fixture.path()).is_err());
    assert_eq!(std::fs::read(fixture.path()).unwrap(), damaged);
}

#[test]
fn failed_replace_keeps_previous_key_and_cleans_encrypted_temp() {
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = Fixture::new();
    let mut connections = Connections::default();
    connections.remember("openai", "https://one.invalid/v1", "model-a", "old-key");
    connections.save(&fixture.path()).unwrap();
    let original = std::fs::read(fixture.path()).unwrap();
    // Deny replacement while allowing reads, as another program might do on Windows.
    let locked = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(fixture.path())
        .unwrap();
    connections.remember("openai", "https://one.invalid/v1", "model-a", "new-key");
    assert!(connections.save(&fixture.path()).is_err());
    assert_eq!(std::fs::read(fixture.path()).unwrap(), original);
    assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), 1);
    drop(locked);
    connections.save(&fixture.path()).unwrap();
    assert_eq!(
        Connections::load(&fixture.path())
            .unwrap()
            .key("openai", "https://one.invalid/v1"),
        "new-key"
    );
}

#[test]
fn oversized_file_is_rejected_before_decryption() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(&fixture.0).unwrap();
    let file = std::fs::File::create(fixture.path()).unwrap();
    file.set_len((LIMIT + 1) as u64).unwrap();
    assert!(Connections::load(&fixture.path()).is_err());
}
