//! Local UI text. Document contents, identifiers and provider responses are never translated.
use serde::{Deserialize, Serialize};
use std::{cell::Cell, collections::BTreeMap, sync::OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    System,
    Korean,
    English,
    Japanese,
}
impl Language {
    pub const ALL: [Self; 4] = [Self::System, Self::Korean, Self::English, Self::Japanese];
    pub fn label(self) -> &'static str {
        match self {
            Self::System => text("시스템 설정 따르기"),
            Self::Korean => "한국어",
            Self::English => "English",
            Self::Japanese => "日本語",
        }
    }
    pub fn resolved(self) -> Self {
        if self != Self::System {
            return self;
        }
        #[cfg(windows)]
        {
            // This reads the Windows display language, not the document's glyph locale.
            match unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() } & 0x3ff {
                0x12 => Self::Korean,
                0x11 => Self::Japanese,
                _ => Self::English,
            }
        }
        #[cfg(not(windows))]
        {
            Self::English
        }
    }
    pub fn english_name(self) -> &'static str {
        match self.resolved() {
            Self::Korean => "Korean",
            Self::Japanese => "Japanese",
            _ => "English",
        }
    }
}
thread_local! { static CURRENT: Cell<Language> = const { Cell::new(Language::Korean) }; }
pub fn set_language(language: Language) {
    CURRENT.set(language.resolved());
}
pub fn current() -> Language {
    CURRENT.get()
}

pub fn catalog() -> &'static BTreeMap<String, [String; 2]> {
    static CATALOG: OnceLock<BTreeMap<String, [String; 2]>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/locales/ui.json")).expect("UI translations")
    })
}
pub fn text(key: &str) -> &str {
    let index = match current() {
        Language::English => 0,
        Language::Japanese => 1,
        _ => return key,
    };
    catalog()
        .get(key)
        .map_or(key, |translations| translations[index].as_str())
}

/// Insert preformatted arguments without scanning or translating their contents.
pub fn format(key: &str, args: &[String]) -> String {
    let template = text(key);
    let mut out =
        String::with_capacity(template.len() + args.iter().map(String::len).sum::<usize>());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if (c == '{' || c == '}') && chars.peek() == Some(&c) {
            chars.next();
            out.push(c);
        } else if c == '{' {
            let mut index = String::new();
            for next in chars.by_ref() {
                if next == '}' {
                    break;
                }
                index.push(next);
            }
            if let Some(arg) = index.parse::<usize>().ok().and_then(|n| args.get(n)) {
                out.push_str(arg);
            } else {
                out.push('{');
                out.push_str(&index);
                out.push('}');
            }
        } else {
            out.push(c);
        }
    }
    out
}
