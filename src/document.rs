use crate::i18n::{format as tf, text as tr};
use crate::model::*;
use std::collections::{HashMap, HashSet};
use unicode_segmentation::UnicodeSegmentation;
pub fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub fn timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
pub fn graphemes(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    text.graphemes(true)
        .filter(|s| seen.insert(*s))
        .map(str::to_owned)
        .collect()
}
pub fn codepoints(text: &str) -> String {
    text.chars()
        .map(|c| format!("U+{:04X}", c as u32))
        .collect::<Vec<_>>()
        .join(" · ")
}
pub fn display(text: &str) -> &str {
    match text {
        " " => "␣",
        "\u{a0}" => "NBSP",
        "\u{3000}" => "全",
        "\t" => "⇥",
        "\n" | "\r\n" => "↵",
        "\u{200d}" => "ZWJ",
        _ => text,
    }
}
pub fn control(text: &str) -> bool {
    text.chars()
        .all(|c| c.is_control() || matches!(c, '\u{200c}' | '\u{200d}' | '\u{2060}' | '\u{feff}'))
}
pub fn status(s: &str) -> &str {
    match s {
        "missing" => tr("미작성"),
        "edited" => tr("작성됨"),
        "blank" => tr("의도적 빈칸"),
        "system" => tr("시스템 표시"),
        _ => tr("미지원"),
    }
}
pub fn new_shape(kind: &str) -> Shape {
    Shape {
        id: uid(),
        name: match kind {
            "path" => tr("자유로운 선"),
            "rect" => tr("네모"),
            "ellipse" => tr("동그라미"),
            _ => tr("가져온 이미지"),
        }
        .into(),
        kind: kind.into(),
        points: vec![],
        contours: vec![],
        x: 150.,
        y: 150.,
        width: 600.,
        height: 600.,
        color: "#34352f".into(),
        stroke: 36.,
        fill: false,
        opacity: 1.,
        visible: true,
        locked: false,
        src: None,
    }
}
pub fn path(pts: &[[f64; 2]]) -> Shape {
    let mut s = new_shape("path");
    s.points = pts.to_vec();
    s
}
pub fn new_glyph(text: &str, locale: &str, group: &str, custom: bool) -> Glyph {
    let cp = text.chars().next().unwrap_or(' ') as u32;
    Glyph {
        id: uid(),
        text: text.into(),
        locale: locale.into(),
        group: group.into(),
        name: display(text).into(),
        mapping: if control(text) {
            "control"
        } else if custom {
            "custom"
        } else if (0xe000..=0xf8ff).contains(&cp) {
            "pua"
        } else if text.chars().count() == 1 {
            "unicode"
        } else {
            "sequence"
        }
        .into(),
        status: if control(text) {
            "unsupported"
        } else {
            "missing"
        }
        .into(),
        locked: false,
        artwork: Artwork {
            shapes: vec![],
            advance: if text == " " || text == "\u{a0}" {
                400.
            } else {
                1000.
            },
        },
        template: None,
        baseline: None,
        candidates: vec![],
        sources: vec![],
    }
}
pub fn editable(g: &Glyph) -> bool {
    !g.locked
        && g.mapping != "control"
        && g.template.as_ref().is_none_or(|t| t.permissions.edit)
        && g.sources.iter().all(|t| t.permissions.edit)
}
pub fn apply_template(g: &mut Glyph, t: &Template) -> Result<(), String> {
    check(
        !g.locked && g.mapping != "control",
        tr("잠긴 글리프 또는 제어 문자는 변경할 수 없어요."),
    )?;
    check(t.permissions.display, tr("템플릿 표시 권한이 없어요."))?;
    let art = t
        .original
        .as_ref()
        .ok_or(tr("복원할 템플릿 원본이 없어요."))?;
    g.template = Some(t.clone());
    g.artwork = art.clone();
    g.baseline = None;
    g.candidates.clear();
    g.sources.clear();
    g.status = "edited".into();
    Ok(())
}
pub fn restore(g: &mut Glyph, mode: &str) -> Result<(), String> {
    check(
        !g.locked && g.mapping != "control",
        tr("잠긴 글리프 또는 제어 문자는 복원할 수 없어요."),
    )?;
    if mode == "system" {
        g.status = "system".into();
        return Ok(());
    }
    let a = if mode == "template" {
        g.template.as_ref().and_then(|t| t.original.as_ref())
    } else {
        g.baseline.as_ref()
    }
    .ok_or(tr(
        "복원할 원본 데이터가 없어요. 같은 버전의 원본을 가져와 주세요.",
    ))?;
    g.artwork = a.clone();
    if mode == "template" {
        g.sources.clear();
    }
    g.status = "edited".into();
    Ok(())
}
pub fn accept(g: &mut Glyph, id: &str) -> Result<(), String> {
    check(
        editable(g) && !matches!(g.status.as_str(), "blank" | "unsupported"),
        tr("잠금·빈칸·미지원 항목은 AI 적용에서 보호돼요."),
    )?;
    let candidate = g
        .candidates
        .iter()
        .find(|c| c.id == id)
        .ok_or(tr("후보를 찾을 수 없어요."))?
        .clone();
    for source in &candidate.sources {
        source.validate()?;
        check(
            source.permissions.edit && source.permissions.ai,
            tr("후보의 사용 권한을 확인해 주세요."),
        )?;
    }
    check(
        g.sources.len()
            + candidate
                .sources
                .iter()
                .filter(|t| !g.sources.iter().any(|s| s.id == t.id))
                .count()
            <= 100,
        tr("참조 출처를 더 저장할 수 없어요."),
    )?;
    g.artwork = candidate.artwork;
    for source in candidate.sources {
        if !g.sources.iter().any(|s| s.id == source.id) {
            g.sources.push(source);
        }
    }
    g.status = "edited".into();
    Ok(())
}
pub fn templates() -> Vec<Template> {
    let a = Artwork {
        advance: 900.,
        shapes: vec![
            path(&[
                [160., 790.],
                [222., 617.],
                [313., 350.],
                [415., 176.],
                [450., 160.],
                [500., 192.],
                [580., 409.],
                [705., 780.],
            ]),
            path(&[[242., 574.], [388., 552.], [565., 560.]]),
        ],
    };
    let mut face = new_shape("ellipse");
    face.x = 160.;
    face.y = 140.;
    face.width = 680.;
    face.height = 680.;
    face.fill = true;
    face.color = "#f6be4f".into();
    let mut left = new_shape("ellipse");
    left.x = 320.;
    left.y = 340.;
    left.width = 44.;
    left.height = 78.;
    left.fill = true;
    let mut right = left.clone();
    right.id = uid();
    right.x = 620.;
    let smile = Artwork {
        advance: 1000.,
        shapes: vec![
            face,
            left,
            right,
            path(&[
                [300., 530.],
                [350., 590.],
                [425., 630.],
                [500., 642.],
                [575., 627.],
                [645., 583.],
                [680., 530.],
            ]),
        ],
    };
    let mut heart = path(&[
        [500., 775.],
        [180., 453.],
        [156., 345.],
        [193., 235.],
        [295., 186.],
        [405., 201.],
        [500., 299.],
        [596., 204.],
        [706., 186.],
        [809., 246.],
        [846., 355.],
        [816., 459.],
    ]);
    heart.fill = true;
    heart.color = "#df795c".into();
    [
        ("doodle-a-v1", tr("연필로 쓴 A"), a),
        ("doodle-smile-v1", tr("햇살 같은 미소"), smile),
        (
            "doodle-heart-v1",
            tr("작은 마음"),
            Artwork {
                advance: 1000.,
                shapes: vec![heart],
            },
        ),
    ]
    .into_iter()
    .map(|(id, name, art)| Template {
        id: id.into(),
        name: name.into(),
        provider: "Doodle Font".into(),
        version: "1.0.0".into(),
        source: "builtin://doodle-font/1.0.0".into(),
        license: "CC0-1.0".into(),
        attribution: "Doodle Font original artwork".into(),
        permissions: Permissions {
            display: true,
            edit: true,
            embed: true,
            redistribute: true,
            ai: true,
        },
        original: Some(art),
    })
    .collect()
}
pub fn new_project() -> Project {
    let ts = templates();
    let mut p = Project {
        schema_version: 1,
        id: uid(),
        name: tr("나의 첫 번째 폰트").into(),
        family: "Doodle Hand".into(),
        created_at: timestamp(),
        updated_at: timestamp(),
        glyphs: vec![],
        templates: ts,
        components: vec![],
        palette: vec![
            "#34352f".into(),
            "#df795c".into(),
            "#f6be4f".into(),
            "#8baf86".into(),
            "#829ec9".into(),
            "#ba96bd".into(),
        ],
    };
    for (text, locale, group, t) in [
        ("A", "default", "latin", 0),
        ("😊", "default", "symbols", 1),
        ("❤️", "default", "symbols", 2),
    ] {
        let mut g = new_glyph(text, locale, group, false);
        apply_template(&mut g, &p.templates[t]).unwrap();
        p.glyphs.push(g)
    }
    p
}
pub const GROUPS: [(&str, &str, &str); 10] = [
    ("latin", "라틴 알파벳", "default"),
    ("latin-ext", "확장 라틴 · 결합 부호", "default"),
    ("hangul", "한글", "ko"),
    ("jamo", "한글 자모 · 옛한글", "ko"),
    ("han-ko", "한국한자", "ko"),
    ("kana", "히라가나 · 가타카나", "ja"),
    ("han-ja", "일본한자", "ja"),
    ("symbols", "기호 · 이모지", "default"),
    ("spaces", "공백 · 제어 문자", "default"),
    ("custom", "사용자 정의", "default"),
];
fn range(a: u32, b: u32) -> Vec<String> {
    (a..=b)
        .filter_map(char::from_u32)
        .map(|c| c.to_string())
        .collect()
}
pub fn catalog(group: &str) -> Vec<String> {
    match group {
        "latin" => [
            range(65, 90),
            range(97, 122),
            range(48, 57),
            range(32, 47),
            range(58, 64),
            range(91, 96),
            range(123, 126),
        ]
        .concat(),
        "latin-ext" => [range(0xc0, 0x17f), range(0x300, 0x36f)].concat(),
        "hangul" => range(0xac00, 0xd7a3),
        "jamo" => [
            range(0x1100, 0x11ff),
            range(0x3131, 0x318e),
            range(0xa960, 0xa97c),
            range(0xd7b0, 0xd7c6),
            range(0xd7cb, 0xd7fb),
        ]
        .concat(),
        "han-ko" | "han-ja" => range(0x4e00, 0x9fff),
        "kana" => [
            range(0x3041, 0x3096),
            range(0x3099, 0x30ff),
            range(0xff61, 0xff9f),
        ]
        .concat(),
        "symbols" => [
            graphemes("♡♥★☆✦✿☀☁☂→←↑↓✓✕∞±×÷=≠😀😊🥰😎😭👍👍🏽❤️👩‍💻👨‍👩‍👧‍👦🌷🌈✨🎉🍀"),
            range(0x2190, 0x21ff),
        ]
        .concat(),
        "spaces" => [
            " ", "\u{a0}", "\u{3000}", "\u{2002}", "\u{2003}", "\u{2009}", "\t", "\n", "\u{200d}",
        ]
        .map(str::to_owned)
        .to_vec(),
        _ => vec![],
    }
}
pub fn add_texts(
    p: &mut Project,
    texts: &[String],
    locale: &str,
    group: &str,
    custom: bool,
) -> usize {
    let mut keys: HashSet<_> = p
        .glyphs
        .iter()
        .map(|g| (g.text.clone(), g.locale.clone()))
        .collect();
    let start = p.glyphs.len();
    for text in texts {
        if !text.is_empty()
            && text.chars().count() <= 128
            && keys.insert((text.clone(), locale.into()))
        {
            p.glyphs.push(new_glyph(text, locale, group, custom))
        }
    }
    p.glyphs.len() - start
}
pub fn smooth(points: &mut [[f64; 2]]) {
    let old = points.to_vec();
    if old.len() > 2 {
        for i in 1..old.len() - 1 {
            points[i] = [
                (old[i - 1][0] + old[i][0] * 2. + old[i + 1][0]) / 4.,
                (old[i - 1][1] + old[i][1] * 2. + old[i + 1][1]) / 4.,
            ]
        }
    }
}
pub fn preview<'a>(
    text: &'a str,
    glyphs: &'a [Glyph],
    locale: &str,
) -> Vec<(&'a str, Option<usize>)> {
    let mut map = HashMap::new();
    for (i, g) in glyphs.iter().enumerate() {
        if g.locale == "default" {
            map.insert(g.text.as_str(), i);
        }
    }
    for (i, g) in glyphs.iter().enumerate() {
        if g.locale == locale {
            map.insert(g.text.as_str(), i);
        }
    }
    let mut custom: Vec<_> = map
        .iter()
        .filter(|(_, i)| glyphs[**i].mapping == "custom")
        .collect();
    custom.sort_by_key(|(s, _)| std::cmp::Reverse(s.len()));
    let mut out = vec![];
    let mut pos = 0;
    while pos < text.len() {
        let rest = &text[pos..];
        let raw = custom
            .iter()
            .find(|(s, _)| rest.starts_with(**s))
            .map(|(s, _)| **s)
            .unwrap_or_else(|| rest.graphemes(true).next().unwrap());
        out.push((&text[pos..pos + raw.len()], map.get(raw).copied()));
        pos += raw.len();
    }
    out
}
pub fn compose_hangul(p: &Project, text: &str) -> Result<(Artwork, Vec<Template>), String> {
    let cp = text
        .chars()
        .next()
        .ok_or(tr("한글 음절을 선택해 주세요."))? as u32;
    check(
        text.chars().count() == 1 && (0xac00..=0xd7a3).contains(&cp),
        tr("완성형 한글 음절을 선택해 주세요."),
    )?;
    let n = cp - 0xac00;
    let l = n / 588;
    let v = n % 588 / 28;
    let t = n % 28;
    let horizontal = [8, 12, 13, 17, 18].contains(&v);
    let body = if t > 0 { 650. } else { 850. };
    let mut shapes = vec![];
    let mut sources: Vec<Template> = vec![];
    let mut parts = vec![
        (
            char::from_u32(0x1100 + l).unwrap(),
            if horizontal {
                [60., 40., 850., body * 0.53]
            } else {
                [30., 40., 500., body]
            },
        ),
        (
            char::from_u32(0x1161 + v).unwrap(),
            if horizontal {
                [60., body * 0.53, 850., body * 0.47]
            } else {
                [520., 40., 440., body]
            },
        ),
    ];
    if t > 0 {
        parts.push((
            char::from_u32(0x11a7 + t).unwrap(),
            [120., 670., 760., 260.],
        ));
    }
    for (c, [x, y, w, h]) in parts {
        let g = p
            .glyphs
            .iter()
            .find(|g| g.text == c.to_string() && g.locale == "ko" && g.status == "edited")
            .ok_or_else(|| {
                tf(
                    "자모 {0} ({1})를 먼저 작성해 주세요.",
                    &[format!("{}", c), codepoints(&c.to_string()).to_string()],
                )
            })?;
        for t in g.template.iter().chain(&g.sources) {
            if !sources.iter().any(|v| v.id == t.id) {
                sources.push(t.clone());
            }
        }
        check(
            g.template
                .as_ref()
                .is_none_or(|t| t.permissions.edit && t.permissions.embed),
            tr("이 자모의 템플릿은 조합 출력 권한을 확인해야 해요."),
        )?;
        for s in &g.artwork.shapes {
            let mut s = s.clone();
            s.id = uid();
            s.x = x + s.x * w / 1000.;
            s.y = y + s.y * h / 1000.;
            s.width *= w / 1000.;
            s.height *= h / 1000.;
            s.stroke *= w.min(h) / 1000.;
            for pt in &mut s.points {
                pt[0] = x + pt[0] * w / 1000.;
                pt[1] = y + pt[1] * h / 1000.;
            }
            shapes.push(s);
        }
    }
    Ok((
        Artwork {
            shapes,
            advance: 1000.,
        },
        sources,
    ))
}
