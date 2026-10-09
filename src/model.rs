use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shape {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub points: Vec<[f64; 2]>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: String,
    pub stroke: f64,
    pub fill: bool,
    pub opacity: f64,
    pub visible: bool,
    pub locked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artwork {
    pub shapes: Vec<Shape>,
    pub advance: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub display: bool,
    pub edit: bool,
    pub embed: bool,
    pub redistribute: bool,
    pub ai: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub version: String,
    pub source: String,
    pub license: String,
    pub attribution: String,
    pub permissions: Permissions,
    pub original: Option<Artwork>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub name: String,
    pub artwork: Artwork,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Glyph {
    pub id: String,
    pub text: String,
    pub locale: String,
    pub group: String,
    pub name: String,
    pub mapping: String,
    pub status: String,
    pub locked: bool,
    pub artwork: Artwork,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<Template>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<Artwork>,
    pub candidates: Vec<Candidate>,
    #[serde(default)]
    pub sources: Vec<Template>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub artwork: Artwork,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<Template>,
    #[serde(default)]
    pub sources: Vec<Template>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub family: String,
    pub created_at: String,
    pub updated_at: String,
    pub glyphs: Vec<Glyph>,
    pub components: Vec<Component>,
    pub templates: Vec<Template>,
    #[serde(default)]
    pub palette: Vec<String>,
}
pub fn check(ok: bool, message: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(message.into())
    }
}
pub fn bounded(s: &str, max: usize) -> bool {
    s.len() <= max
}
impl Artwork {
    pub fn validate(&self) -> Result<(), String> {
        check(
            self.advance.is_finite()
                && (0.0..=4000.0).contains(&self.advance)
                && self.shapes.len() <= 5000,
            "글리프 폭 또는 레이어 개수를 확인해 주세요.",
        )?;
        let mut ids = HashSet::new();
        for s in &self.shapes {
            check(
                ids.insert(&s.id) && bounded(&s.id, 100) && bounded(&s.name, 600),
                "레이어 ID가 중복되었어요.",
            )?;
            check(
                ["path", "rect", "ellipse", "image"].contains(&s.kind.as_str()),
                "지원하지 않는 레이어예요.",
            )?;
            check(
                [s.x, s.y, s.width, s.height, s.stroke, s.opacity]
                    .iter()
                    .all(|v| v.is_finite())
                    && s.x.abs() <= 4000.
                    && s.y.abs() <= 4000.
                    && (0.0..=4000.).contains(&s.width)
                    && (0.0..=4000.).contains(&s.height)
                    && (0.0..=300.).contains(&s.stroke)
                    && (0.0..=1.).contains(&s.opacity),
                "그림 좌표·크기 범위를 확인해 주세요.",
            )?;
            check(
                s.color.len() == 7
                    && s.color.starts_with('#')
                    && s.color[1..].bytes().all(|c| c.is_ascii_hexdigit()),
                "색상은 #RRGGBB 형식이어야 해요.",
            )?;
            check(
                s.points.len() <= 10000
                    && s.points
                        .iter()
                        .flatten()
                        .all(|v| v.is_finite() && v.abs() <= 4000.),
                "선의 좌표를 확인해 주세요.",
            )?;
            if s.kind == "image" {
                check(
                    s.src.as_ref().is_some_and(|v| {
                        v.starts_with("data:image/png;base64,") && v.len() <= 12_000_000
                    }),
                    "8MB 이하 PNG 이미지를 사용해 주세요.",
                )?;
            }
        }
        Ok(())
    }
}
impl Template {
    pub fn validate(&self) -> Result<(), String> {
        check(
            [
                &self.id,
                &self.name,
                &self.provider,
                &self.version,
                &self.source,
                &self.license,
                &self.attribution,
            ]
            .iter()
            .all(|s| bounded(s, 4000)),
            "템플릿 정보가 너무 길어요.",
        )?;
        if let Some(a) = &self.original {
            a.validate()?;
        }
        Ok(())
    }
}
impl Project {
    pub fn validate(&self) -> Result<(), String> {
        check(
            self.schema_version == 1,
            "Doodle Font 버전 1 프로젝트가 아니에요.",
        )?;
        check(
            !self.family.trim().is_empty()
                && [
                    &self.id,
                    &self.name,
                    &self.family,
                    &self.created_at,
                    &self.updated_at,
                ]
                .iter()
                .all(|s| bounded(s, 600)),
            "프로젝트 이름을 확인해 주세요.",
        )?;
        check(
            self.glyphs.len() <= 150000
                && self.templates.len() <= 1000
                && self.components.len() <= 1000,
            "프로젝트 항목이 너무 많아요.",
        )?;
        let mut ids = HashSet::new();
        let mut maps = HashSet::new();
        check(
            self.palette.len() <= 64
                && self.palette.iter().all(|c| {
                    c.len() == 7
                        && c.starts_with('#')
                        && c[1..].bytes().all(|v| v.is_ascii_hexdigit())
                }),
            "팔레트에는 최대 64개의 #RRGGBB 색상을 보관해요.",
        )?;
        for g in &self.glyphs {
            check(
                ids.insert(&g.id) && maps.insert((&g.text, &g.locale)),
                "글리프 ID 또는 문자·지역 매핑이 중복되었어요.",
            )?;
            check(
                !g.text.is_empty()
                    && g.id.len() <= 100
                    && g.name.len() <= 600
                    && g.group.len() <= 100
                    && g.text.chars().count() <= 128
                    && ["default", "ko", "ja"].contains(&g.locale.as_str()),
                "문자 매핑을 확인해 주세요.",
            )?;
            check(
                ["missing", "edited", "blank", "system", "unsupported"]
                    .contains(&g.status.as_str())
                    && ["unicode", "sequence", "custom", "pua", "control"]
                        .contains(&g.mapping.as_str()),
                "글리프 상태를 확인해 주세요.",
            )?;
            check(
                g.candidates.len() <= 50,
                "AI 후보는 최대 50개까지 보관해요.",
            )?;
            g.artwork.validate()?;
            check(
                g.sources.len() <= 100,
                "참조 템플릿은 글리프당 100개 이하여야 해요.",
            )?;
            for t in &g.sources {
                t.validate()?;
            }
            check(
                !crate::document::control(&g.text)
                    || (g.mapping == "control" && g.status == "unsupported"),
                "제어 문자는 확인 전용 상태로 보관해야 해요.",
            )?;
            if let Some(t) = &g.template {
                t.validate()?;
            }
            if let Some(a) = &g.baseline {
                a.validate()?;
            }
            for c in &g.candidates {
                c.artwork.validate()?;
            }
        }
        for t in &self.templates {
            t.validate()?;
        }
        for c in &self.components {
            for t in &c.sources {
                t.validate()?;
            }
            c.artwork.validate()?;
            if let Some(t) = &c.template {
                t.validate()?;
            }
        }
        Ok(())
    }
}
