//! Check desktop exports with the loader used by Windows font consumers.
use windows_sys::Win32::Graphics::Gdi::{AddFontMemResourceEx, RemoveFontMemResourceEx};

pub(super) fn validate(bytes: &[u8]) -> Result<(), String> {
    let length = u32::try_from(bytes.len())
        .map_err(|_| "Windows에서 읽기에 폰트가 너무 커요. 분할 출력해 주세요.")?;
    let mut count = 0u32;
    // The buffer remains alive during the call; Windows copies it on success.
    // Memory fonts are private to this process and are never installed system-wide.
    let handle = unsafe {
        // The Windows metadata types pNumFonts as const, but the API writes to it.
        AddFontMemResourceEx(
            bytes.as_ptr().cast(),
            length,
            std::ptr::null(),
            &raw mut count,
        )
    };
    if handle.is_null() {
        return Err("Windows에서 이 폰트를 읽지 못했어요. 파일을 저장하지 않았습니다. 출력 범위를 줄여 다시 시도해 주세요.".into());
    }
    // Release the temporary font before returning, including the unexpected zero-count case.
    unsafe { RemoveFontMemResourceEx(handle) };
    super::check(
        count > 0,
        "Windows에서 출력 폰트의 글꼴 정보를 확인하지 못했어요.",
    )
}
