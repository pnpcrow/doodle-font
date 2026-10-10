//! Typed worker events: model output can never inject an arbitrary animation command.
use crate::companion::Mood;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    Message(String),
    Animation(Mood),
}
impl From<&str> for Progress {
    fn from(value: &str) -> Self {
        Self::Message(value.into())
    }
}
impl From<String> for Progress {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}
impl std::fmt::Display for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(s) => f.write_str(s),
            Self::Animation(m) => f.write_str(m.label()),
        }
    }
}
