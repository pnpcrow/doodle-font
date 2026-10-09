use crate::model::{Artwork, Glyph};
use serde_json::{json, Value};
pub async fn generate(payload: Value) -> Result<Artwork, String> {
    let g: Glyph = serde_json::from_value(payload["glyph"].clone()).map_err(|e| e.to_string())?;
    if g.locked || ["blank", "unsupported"].contains(&g.status.as_str()) || g.mapping == "control" {
        return Err("잠금·빈칸·제어 문자·미지원 항목은 AI 생성에서 보호돼요.".into());
    }
    if g.template
        .iter()
        .chain(&g.sources)
        .any(|t| !t.permissions.ai || !t.permissions.edit)
    {
        return Err("템플릿의 AI 전송·편집 권한이 없어요.".into());
    }
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
    let reference = if payload["sendReference"] == true {
        let mut a = g.artwork.clone();
        a.shapes.retain(|s| s.kind != "image");
        Some(a)
    } else {
        None
    };
    let context = json!({"exactText":g.text,"codepoints":g.text.chars().map(|c|format!("U+{:04X}",c as u32)).collect::<Vec<_>>(),"locale":g.locale,"colorMode":payload["colorMode"],"style":payload["prompt"],"reference":reference,"protected":"Preserve text mapping, template original, baseline and all other variants."});
    let prompt=format!("Design a glyph as JSON only: {{\"advance\":1000,\"shapes\":[{{\"type\":\"path\",\"points\":[[100,800],[500,200],[800,800]],\"x\":0,\"y\":0,\"width\":0,\"height\":0,\"color\":\"#34352f\",\"stroke\":40,\"fill\":false,\"opacity\":1}}]}}. Allowed types: path, rect, ellipse. Coordinates 0..1000, baseline y=800. Max 64 shapes, 500 points per path. No code or images. Context: {context}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let request = if provider == "anthropic" {
        client.post(format!("{endpoint}/messages")).header("x-api-key",key).header("anthropic-version","2023-06-01").json(&json!({"model":model,"max_tokens":4096,"messages":[{"role":"user","content":prompt}]}))
    } else {
        client
            .post(format!("{endpoint}/chat/completions"))
            .bearer_auth(key)
            .json(&json!({"model":model,"messages":[{"role":"user","content":prompt}]}))
    };
    let mut response = request
        .send()
        .await
        .map_err(|_| "API에 연결하지 못했어요. 주소와 네트워크를 확인해 주세요.")?;
    if !response.status().is_success() {
        return Err(format!(
            "API 응답 오류 ({}). 주소·모델·키를 확인해 주세요.",
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
    let answer = if provider == "anthropic" {
        result["content"]
            .as_array()
            .ok_or("AI 응답이 비어 있어요.")?
            .iter()
            .filter_map(|c| c["text"].as_str())
            .collect::<String>()
    } else {
        result["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("AI 응답이 비어 있어요.")?
            .to_owned()
    };
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
