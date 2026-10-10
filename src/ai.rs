use crate::model::{Artwork, Glyph};
use serde_json::{json, Value};
pub fn validate_target(payload: &Value, editing: bool) -> Result<Glyph, String> {
    let g: Glyph = serde_json::from_value(payload["glyph"].clone()).map_err(|e| e.to_string())?;
    if editing
        && (g.locked
            || ["blank", "unsupported"].contains(&g.status.as_str())
            || g.mapping == "control")
    {
        return Err("잠금·빈칸·제어 문자·미지원 항목은 AI 생성에서 보호돼요.".into());
    }
    if g.template
        .iter()
        .chain(&g.sources)
        .any(|t| !t.permissions.ai || editing && !t.permissions.edit)
    {
        return Err("템플릿의 AI 전송·편집 권한이 없어요.".into());
    }
    if payload["prompt"].as_str().unwrap_or("").len() > 8000 {
        return Err("프롬프트는 8,000바이트 이하여야 해요.".into());
    }
    Ok(g)
}
pub fn canvas_image(artwork: &Artwork) -> Result<String, String> {
    use crate::glyph_layout::{self, EM, SNAPSHOT};
    use base64::Engine;
    let image = crate::font::raster(artwork, SNAPSHOT)?;
    let mut white =
        image::RgbaImage::from_pixel(SNAPSHOT, SNAPSHOT, image::Rgba([255, 255, 255, 255]));
    for (y, _, [r, g, b]) in glyph_layout::guides() {
        let y = (y * SNAPSHOT as f64 / EM).round() as u32;
        for x in 0..SNAPSHOT {
            white.put_pixel(x, y, image::Rgba([r, g, b, 255]));
        }
    }
    for x in glyph_layout::vertical_guides(artwork.advance) {
        let x = ((x * SNAPSHOT as f64 / EM).round() as u32).min(SNAPSHOT - 1);
        for y in 1..SNAPSHOT {
            white.put_pixel(x, y, image::Rgba([161, 181, 156, 255]));
        }
    }
    image::imageops::overlay(&mut white, &image, 0, 0);
    let mut bytes = std::io::Cursor::new(Vec::new());
    white
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
    ))
}
pub async fn assist(payload: Value) -> Result<String, String> {
    let g = validate_target(&payload, false)?;
    let layout = crate::glyph_layout::context(&g);
    let prompt=format!("You are Doodle Font's font drawing assistant. Reply in Korean, concise and specific. Explain what you see and suggest changes relevant to the user's request. The pale horizontal/vertical lines are editing guides, not part of the glyph. If an image is attached, its 512 pixels represent 1000 font units, not 512 font units. Keep cap height, lowercase proportions and descenders distinct; do not move a descender above the baseline. Do not claim to have edited anything. Layout: {layout}. Glyph: {}, locale: {}, codepoints: {:?}. User request: {}",g.text,g.locale,g.text.chars().map(|c|format!("U+{:04X}",c as u32)).collect::<Vec<_>>(),payload["prompt"].as_str().unwrap_or(""));
    request(&payload, &g, &prompt, false, None).await
}
async fn request(
    payload: &Value,
    g: &Glyph,
    prompt: &str,
    drawing: bool,
    schema: Option<Value>,
) -> Result<String, String> {
    let config = &payload["config"];
    let endpoint = config["endpoint"]
        .as_str()
        .unwrap_or("")
        .trim_end_matches('/');
    let url = reqwest::Url::parse(endpoint).map_err(|_| "API 주소를 확인해 주세요.")?;
    if !(url.scheme() == "https"
        || url.scheme() == "http"
            && ["localhost", "127.0.0.1", "[::1]"].contains(&url.host_str().unwrap_or("")))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("외부 API에는 HTTPS 주소를 사용해 주세요.".into());
    }
    let model = config["model"].as_str().unwrap_or("");
    let key = config["key"].as_str().unwrap_or("");
    let provider = config["provider"].as_str().unwrap_or("");
    if model.trim().is_empty() || !["openai", "anthropic"].contains(&provider) {
        return Err("API 방식과 모델 이름을 입력해 주세요.".into());
    }
    let picture = if payload["sendReference"] == true {
        Some(canvas_image(&g.artwork)?)
    } else {
        None
    };
    let content = if let Some(picture) = picture {
        if provider == "anthropic" {
            json!([{"type":"image","source":{"type":"base64","media_type":"image/png","data":picture.trim_start_matches("data:image/png;base64,")}},{"type":"text","text":prompt}])
        } else {
            json!([{"type":"image_url","image_url":{"url":picture}},{"type":"text","text":prompt}])
        }
    } else {
        json!(prompt)
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(if config["local"] == true {
            300
        } else {
            90
        }))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    let request = if provider == "anthropic" {
        client.post(format!("{endpoint}/messages")).header("x-api-key",key).header("anthropic-version","2023-06-01").json(&json!({"model":model,"max_tokens":4096,"messages":[{"role":"user","content":content}]}))
    } else {
        let mut body = json!({"model":model,"max_tokens":if schema.is_some() {512} else if drawing {4096} else {768},"temperature":0.3,"messages":[{"role":"user","content":content}]});
        if config["local"] == true {
            body["chat_template_kwargs"] = json!({"enable_thinking":false});
            if let Some(schema) = schema {
                body["response_format"] = json!({"type":"json_object","schema":schema});
            } else if drawing {
                body["response_format"] = json!({"type":"json_object","schema":{
                "type":"object","required":["advance","shapes"],"additionalProperties":false,
                "properties":{"advance":{"type":"number","minimum":0,"maximum":2000},"shapes":{"type":"array","minItems":1,"maxItems":12,"items":{
                    "type":"object","additionalProperties":false,"required":["type","x","y","width","height","points","color","stroke","fill","opacity"],
                    "properties":{"type":{"type":"string","enum":["path","rect","ellipse"]},"x":{"type":"number"},"y":{"type":"number"},"width":{"type":"number","minimum":1},"height":{"type":"number","minimum":1},"points":{"type":"array","maxItems":100,"items":{"type":"array","minItems":2,"maxItems":2,"items":{"type":"number"}}},"color":{"type":"string","pattern":"^#[0-9a-fA-F]{6}$"},"stroke":{"type":"number","minimum":0,"maximum":200},"fill":{"type":"boolean"},"opacity":{"type":"number","minimum":0,"maximum":1}}
                }}}}});
            }
        }
        client
            .post(format!("{endpoint}/chat/completions"))
            .bearer_auth(key)
            .json(&body)
    };
    let mut response = request
        .send()
        .await
        .map_err(|_| "AI에 연결하지 못했어요. 주소와 네트워크를 확인해 주세요.")?;
    if !response.status().is_success() {
        return Err(format!(
            "AI 응답 오류 ({}). 주소·모델·키를 확인해 주세요.",
            response.status().as_u16()
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "AI 응답을 읽지 못했어요.")?
    {
        if bytes.len() + chunk.len() > 2_000_000 {
            return Err("AI 응답이 너무 커요.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let result: Value =
        serde_json::from_slice(&bytes).map_err(|_| "AI 응답이 JSON 형식이 아니에요.")?;
    if provider == "anthropic" {
        Ok(result["content"]
            .as_array()
            .ok_or("AI 응답이 비어 있어요.")?
            .iter()
            .filter_map(|c| c["text"].as_str())
            .collect())
    } else {
        result["choices"][0]["message"]["content"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned)
            .ok_or_else(|| "AI 응답이 비어 있어요.".into())
    }
}
pub async fn generate(payload: Value) -> Result<Artwork, String> {
    let g = validate_target(&payload, true)?;
    let reference = if payload["sendReference"] == true {
        let mut a = g.artwork.clone();
        a.shapes.retain(|s| s.kind != "image");
        Some(a)
    } else {
        None
    };
    let context = json!({"exactText":g.text,"codepoints":g.text.chars().map(|c|format!("U+{:04X}",c as u32)).collect::<Vec<_>>(),"locale":g.locale,"colorMode":payload["colorMode"],"style":payload["prompt"],"reference":reference,"layout":crate::glyph_layout::context(&g),"protected":"Preserve text mapping, template original, baseline and all other variants."});
    let prompt=format!("Create a vector drawing of the exact target glyph, matching the reference image and the user's style instruction. Return JSON only with numeric advance and an array shapes. Each shape must have type (path, rect or ellipse), color (#RRGGBB), stroke (numeric width), fill (boolean), opacity (0..1). Paths have points: an array of numeric [x,y] pairs. Rectangles and ellipses have x,y,width,height (bounding box, not center or radii). Follow the supplied layout in font units, not image pixels. Baseline y=800 is the bottom of the main letter body, NOT the bottom of every glyph: g/j/p/q/y need strokes below y=800. Capital tops are near y=200; lowercase bodies stay smaller and keep their x-height. The y=500 midline is only a visual reference. Preserve natural overshoot, accents and punctuation position. Before drawing, use layout.naturalReference as an approximate size and aspect-ratio starting point for this exact text, then adapt the style. The 1000-unit canvas is NOT the letter bounding box. For new letters balance the visible ink horizontally around layout.spacing.centerX in the existing advance. Left and right ink limits are minimum margins, NOT target endpoints: leave generous natural whitespace instead of touching both limits. Never anchor all letters to the left limit. Do not stretch, compress, or independently normalize width and height to fill the area; I/l/i stay narrow, M/W stay wide, and lowercase/punctuation stay naturally smaller. Choose the main body height from the cap/x-height guides, then let the letter design determine its width. Measure the actual visible ink including half the stroke at path edges. Do not vertically center or shrink a descender onto the baseline. Keep the existing advance unchanged. Check proportions, balanced side margins and baseline before returning JSON. For requested existing-art edits or explicit positioning, preserve that placement. Pale guide lines in the reference are NOT glyph strokes; never trace them. Use at most 12 simple shapes and 100 points per path. No prose, code, SVG or nested JSON strings. Draw the requested glyph, not a generic letter or demonstration. Context: {context}");
    let answer = request(&payload, &g, &prompt, true, None).await?;
    let cleaned = answer
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let mut data: Value =
        serde_json::from_str(cleaned).map_err(|_| "AI가 올바른 그림을 반환하지 않았어요.")?;
    let shapes = data["shapes"]
        .as_array_mut()
        .ok_or("AI 그림에 레이어가 없어요.")?;
    if shapes.len() > 64 {
        return Err("AI 레이어가 너무 많아요.".into());
    }
    for s in shapes {
        let o = s.as_object_mut().ok_or("AI 레이어 형식 오류")?;
        if o.get("type")
            .and_then(Value::as_str)
            .is_some_and(|t| ["rect", "ellipse"].contains(&t))
            && ["width", "height"]
                .iter()
                .any(|k| o.get(*k).and_then(Value::as_f64).is_none_or(|n| n <= 0.))
        {
            return Err("AI 도형의 가로·세로 크기가 올바르지 않아요. 다시 요청해 주세요.".into());
        }
        let defaults = json!({"id":uuid::Uuid::new_v4().to_string(),"name":"AI 후보 레이어","points":[],"x":0,"y":0,"width":0,"height":0,"color":"#34352f","stroke":36,"fill":false,"opacity":1,"visible":true,"locked":false});
        for (k, v) in defaults.as_object().unwrap() {
            o.entry(k.clone()).or_insert(v.clone());
        }
        if !o
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|v| ["path", "rect", "ellipse"].contains(&v))
        {
            return Err("AI 레이어는 path·rect·ellipse 형식이어야 해요.".into());
        }
        if o.get("points")
            .and_then(Value::as_array)
            .is_none_or(|v| v.len() > 500)
        {
            return Err("AI 선의 점은 500개 이하여야 해요.".into());
        }
        o.insert("id".into(), json!(uuid::Uuid::new_v4().to_string()));
    }
    if data.get("advance").is_none() {
        data["advance"] = json!(g.artwork.advance)
    }
    let mut a: Artwork = serde_json::from_value(data).map_err(|_| "AI 그림 형식이 맞지 않아요.")?;
    validate_candidate(&a)?;
    crate::glyph_layout::place_api_candidate(&g, payload["prompt"].as_str().unwrap_or(""), &mut a)?;
    validate_candidate(&a)?;
    Ok(a)
}

/// A drawing request must produce something the user can actually preview.
pub fn validate_candidate(artwork: &Artwork) -> Result<(), String> {
    artwork.validate()?;
    if artwork.shapes.is_empty()
        || !crate::font::raster(artwork, 512)?
            .pixels()
            .any(|pixel| pixel[3] > 0)
    {
        return Err(
            "AI가 보이는 그림을 반환하지 않았어요. 후보를 저장하지 않았습니다. 다시 만들어 주세요."
                .into(),
        );
    }
    Ok(())
}

pub async fn generate_typography(payload: Value) -> Result<Value, String> {
    let glyph = validate_target(&payload, true)?;
    let prompt = format!(
        r##"You are a typography style planner. A deterministic font engine will draw the EXACT text {:?}, preserving every character and counter. Do NOT invent points or paths. Read the user's Korean instruction and the current canvas. Return ONLY the style JSON.
family: sans for 고딕/둥근/monoline, serif for 명조/serif, handwritten for 손글씨/handwriting.
weight: 200 thin, 400 regular, 650 bold, 800 very bold. width: 100 normal, 85 narrow, 115 wide. Always use width=100 and tracking=0 unless the user explicitly requests a width or spacing change; bold/round/handwritten does NOT mean wide. Never vary width to fill the canvas or approach its guide limits. slant: 0 upright, 10 right italic, negative left. roundness: 0 sharp, 80 rounded. tracking: 0 normal. color: #RRGGBB (default #242424).
source is an editing intent, not a font name. The Korean phrases 현재 그림을 유지, 형태와 위치는 그대로, 기존 그림 보정 mean source=current. When the user asks to KEEP the current drawing you MUST choose current, even if the target is a normal letter. Otherwise choose glyph to draw the correct text. stroke: one consistent width for all strokes when source=current; default 48, bold 70, thin 24. Never change the requested text.
Example: 기존 그림은 그대로 두고 획 두께만 60으로 맞춰줘 -> {{"family":"sans","weight":500,"width":100,"slant":0,"roundness":0,"tracking":0,"color":"#242424","source":"current","stroke":60}}.
Examples: 굵고 둥근 고딕체 -> {{"family":"sans","weight":650,"width":100,"slant":0,"roundness":80,"tracking":0,"color":"#242424","source":"glyph","stroke":70}}.
The deterministic engine balances fresh glyphs horizontally within the advance without resizing them to the box, and keeps the baseline and descenders. It keeps the reference position for source=current. Natural proportions and whitespace are intentional: I is narrower than M and lowercase/punctuation are not enlarged to capital height. Pale image guides are not glyph strokes. Layout: {}.
얇은 명조체 -> family serif, weight 250, roundness 0. 손글씨 -> family handwritten. User instruction: {}"##,
        glyph.text,
        crate::glyph_layout::context(&glyph),
        payload["prompt"].as_str().unwrap_or("")
    );
    let answer = request(
        &payload,
        &glyph,
        &prompt,
        true,
        Some(crate::typography::schema()),
    )
    .await?;
    let style: crate::typography::Style = serde_json::from_str(answer.trim())
        .map_err(|_| "AI가 글자 스타일을 올바르게 반환하지 않았어요. 다시 요청해 주세요.")?;
    let generated = crate::typography::generate(&glyph, &style, payload["colorMode"] == "mono")?;
    Ok(
        json!({"artwork":generated.artwork,"sources":generated.sources,"generationNote":generated.note,"style":style}),
    )
}
