//! User-scoped, encrypted connection settings. Never part of a font project.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Write, path::Path};

const HEADER: &[u8] = b"DoodleFont-DPAPI-1\0";
const LIMIT: usize = 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    pub endpoint: String,
    pub model: String,
}

// Deliberately no Debug implementation: this object contains credentials.
#[derive(Serialize, Deserialize)]
pub struct Connections {
    pub provider: String,
    profiles: BTreeMap<String, Profile>,
    keys: BTreeMap<String, BTreeMap<String, String>>,
}
impl Default for Connections {
    fn default() -> Self {
        Self {
            provider: "off".into(),
            profiles: [
                ("openai", "https://api.openai.com/v1"),
                ("anthropic", "https://api.anthropic.com/v1"),
            ]
            .into_iter()
            .map(|(provider, endpoint)| {
                (
                    provider.into(),
                    Profile {
                        endpoint: endpoint.into(),
                        model: String::new(),
                    },
                )
            })
            .collect(),
            keys: BTreeMap::new(),
        }
    }
}
fn address(endpoint: &str) -> &str {
    endpoint.trim().trim_end_matches('/')
}
impl Connections {
    pub fn profile(&self, provider: &str) -> Profile {
        self.profiles.get(provider).cloned().unwrap_or(Profile {
            endpoint: String::new(),
            model: String::new(),
        })
    }
    pub fn key(&self, provider: &str, endpoint: &str) -> String {
        self.keys
            .get(provider)
            .and_then(|keys| keys.get(address(endpoint)))
            .cloned()
            .unwrap_or_default()
    }
    pub fn remember(&mut self, provider: &str, endpoint: &str, model: &str, key: &str) {
        self.provider = provider.into();
        if !["openai", "anthropic"].contains(&provider) {
            return;
        }
        self.profiles.insert(
            provider.into(),
            Profile {
                endpoint: endpoint.trim().into(),
                model: model.trim().into(),
            },
        );
        let keys = self.keys.entry(provider.into()).or_default();
        if key.trim().is_empty() {
            keys.remove(address(endpoint));
        } else {
            keys.insert(address(endpoint).into(), key.trim().into());
        }
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        if std::fs::metadata(path).is_ok_and(|m| m.len() > LIMIT as u64) {
            return Err("AI 연결 설정 파일이 너무 커요.".into());
        }
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(_) => {
                return Err("AI 연결 설정을 읽지 못했어요. 파일 접근 권한을 확인해 주세요.".into())
            }
        };
        if bytes.len() > LIMIT || !bytes.starts_with(HEADER) {
            return Err("AI 연결 설정 파일이 손상됐어요. 기존 파일은 보존합니다.".into());
        }
        let plaintext = protect(&bytes[HEADER.len()..], false)?;
        // Never include a serde error: it may quote a credential from a bad file.
        let settings: Self = serde_json::from_slice(&plaintext)
            .map_err(|_| "AI 연결 설정 형식이 맞지 않아요. 기존 파일은 보존합니다.".to_string())?;
        if !["off", "local", "openai", "anthropic"].contains(&settings.provider.as_str()) {
            return Err("저장된 AI 제공자를 확인할 수 없어요.".into());
        }
        Ok(settings)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let plaintext = serde_json::to_vec(self)
            .map_err(|_| "AI 연결 설정을 준비하지 못했어요.".to_string())?;
        if plaintext.len() > LIMIT / 2 {
            return Err("AI 연결 설정이 너무 커요.".into());
        }
        let encrypted = protect(&plaintext, true)?;
        let parent = path.parent().ok_or("AI 설정 폴더를 찾지 못했어요.")?;
        std::fs::create_dir_all(parent).map_err(|_| "AI 설정 폴더를 만들지 못했어요.")?;
        // The only on-disk temporary data is encrypted too. Failed writes preserve the last save.
        let pending = path.with_extension(format!("{}.pending", uuid::Uuid::new_v4()));
        let result = (|| -> std::io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&pending)?;
            file.write_all(HEADER)?;
            file.write_all(&encrypted)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&pending, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&pending);
        }
        result.map_err(|_| {
            "AI 연결 설정을 저장하지 못했어요. 폴더 권한을 확인하고 다시 저장해 주세요.".into()
        })
    }
}

#[cfg(windows)]
fn protect(bytes: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len().try_into().map_err(|_| "AI 설정이 너무 커요.")?,
        pbData: bytes.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: DPAPI reads the live input slice and allocates the output using LocalAlloc.
    // No machine-wide flag: only the current Windows user's DPAPI identity can decrypt it.
    let ok = unsafe {
        if encrypt {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err(if encrypt { "Windows가 API 키를 암호화하지 못했어요. 키는 이번 실행에서만 사용됩니다." } else { "저장된 AI 연결을 복원하지 못했어요. 저장한 Windows 계정에서 실행하거나 연결 정보를 다시 입력해 주세요." }.into());
    }
    // SAFETY: a successful call owns this cbData-sized buffer; release it exactly once.
    let result = unsafe {
        let value = if output.cbData == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec()
        };
        LocalFree(output.pbData.cast());
        value
    };
    Ok(result)
}
#[cfg(not(windows))]
fn protect(_: &[u8], _: bool) -> Result<Vec<u8>, String> {
    Err("API 키의 암호화 저장은 Windows에서 사용할 수 있어요.".into())
}

#[cfg(all(test, windows))]
#[path = "ai_settings_tests.rs"]
mod tests;
