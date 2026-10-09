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
    use base64::Engine;
    let image = crate::font::raster(artwork, 512)?;
    let mut white = image::RgbaImage::from_pixel(512, 512, image::Rgba([255, 255, 255, 255]));
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
    let prompt=format!("You are Doodle Font's font drawing assistant. Reply in Korean, concise and specific. The image is the current glyph canvas, coordinates 0..1000, baseline y=800. Explain what you see and suggest changes relevant to the user's request. Do not claim to have edited anything. Glyph: {}, locale: {}, codepoints: {:?}. User request: {}",g.text,g.locale,g.text.chars().map(|c|format!("U+{:04X}",c as u32)).collect::<Vec<_>>(),payload["prompt"].as_str().unwrap_or(""));
    request(&payload, &g, &prompt, false).await
}
async fn request(
    payload: &Value,
    g: &Glyph,
    prompt: &str,
    drawing: bool,
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
            json!([{"type":"text","text":prompt},{"type":"image_url","image_url":{"url":picture}}])
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
        let mut body = json!({"model":model,"max_tokens":if drawing {4096} else {768},"temperature":0.3,"messages":[{"role":"user","content":content}]});
        if config["local"] == true {
            body["chat_template_kwargs"] = json!({"enable_thinking":false});
            if drawing {
                body["response_format"] = json!({"type":"json_object","schema":{
                "type":"object","required":["advance","shapes"],"additionalProperties":false,
                "properties":{"advance":{"type":"number","minimum":0,"maximum":2000},"shapes":{"type":"array","maxItems":12,"items":{
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
    let context = json!({"exactText":g.text,"codepoints":g.text.chars().map(|c|format!("U+{:04X}",c as u32)).collect::<Vec<_>>(),"locale":g.locale,"colorMode":payload["colorMode"],"style":payload["prompt"],"reference":reference,"protected":"Preserve text mapping, template original, baseline and all other variants."});
    let prompt=format!("Create a vector drawing of the exact target glyph, matching the reference image and the user's style instruction. Return JSON only with numeric advance and an array shapes. Each shape must have type (path, rect or ellipse), color (#RRGGBB), stroke (numeric width), fill (boolean), opacity (0..1). Paths have points: an array of numeric [x,y] pairs. Rectangles and ellipses have x,y,width,height (bounding box, not center or radii). Use coordinates 0..1000 with baseline y=800. Use at most 12 simple shapes and 100 points per path. No prose, code, SVG or nested JSON strings. Draw the requested glyph, not a generic letter or demonstration. Context: {context}");
    let answer = request(&payload, &g, &prompt, true).await?;
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
    let a: Artwork = serde_json::from_value(data).map_err(|_| "AI 그림 형식이 맞지 않아요.")?;
    a.validate()?;
    Ok(a)
}
