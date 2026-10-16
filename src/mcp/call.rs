//! One tool call: arguments → HTTP request → the API router (in memory) → MCP result.

use axum::{
    Router,
    body::{Body, Bytes},
    http::{HeaderValue, Method, Request, StatusCode, header},
};
use rmcp::{
    ErrorData,
    model::{CallToolResult, ContentBlock},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use url::Url;
use uuid::Uuid;

use super::{
    schema::{ArgMap, CONFIRM_ARG, ITEMS_ARG, JsonObject},
    tools::{BodyKind, Kind, TOOLS, ToolDef},
};

/// The CSV export of a large audience is the biggest answer; 16 MB is far above it.
const MAX_ANSWER_BYTES: usize = 16 * 1024 * 1024;

struct Answer {
    status: StatusCode,
    content_type: String,
    bytes: Bytes,
}

/// Runs a tool call through the API router as the caller. Destructive tools without `confirm: true` only preview.
pub async fn run(
    api: &Router,
    auth: &HeaderValue,
    def: &ToolDef,
    map: &ArgMap,
    args: JsonObject,
) -> Result<CallToolResult, ErrorData> {
    if def.kind == Kind::Destructive && args.get(CONFIRM_ARG) != Some(&Value::Bool(true)) {
        return preview(api, auth, def, &args).await;
    }
    let body = match def.body {
        BodyKind::CsvUpload => Some(csv_upload(&args)?),
        BodyKind::Json => json_body(map, &args)
            .map(|v| ("application/json".to_string(), v.to_string().into_bytes())),
    };
    let req = request(def.method, uri(def.path, &args, &map.query)?, body, auth)?;
    Ok(answer(def, send(api, req).await?))
}

/// Shows what a destructive call would do, without doing it.
async fn preview(
    api: &Router,
    auth: &HeaderValue,
    def: &ToolDef,
    args: &JsonObject,
) -> Result<CallToolResult, ErrorData> {
    let mut lines = vec![format!(
        "Preview only. Nothing changed yet. This will {}.",
        def.effect
    )];
    match def.preview_get {
        Some(get) => {
            let a = send(api, request("get", uri(get, args, &[])?, None, auth)?).await?;
            if !a.status.is_success() {
                return Ok(answer(def, a));
            }
            lines.push(format!("Target: {}", String::from_utf8_lossy(&a.bytes)));
        }
        None => {
            let target: JsonObject = args
                .iter()
                .filter(|(k, _)| k.as_str() != CONFIRM_ARG)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            lines.push(format!("Target: {}", Value::Object(target)));
        }
    }
    if def.name == "send_campaign" {
        let a = send(
            api,
            request(
                "post",
                uri("/campaigns/{id}/preview", args, &[])?,
                None,
                auth,
            )?,
        )
        .await?;
        if a.status.is_success() {
            let p: Value = serde_json::from_slice(&a.bytes).unwrap_or(Value::Null);
            lines.push(format!("Readers who will get it: {}.", p["readers"]));
            if let Some(w) = p["warnings"].as_array().filter(|w| !w.is_empty()) {
                lines.push(format!("Warnings: {}", Value::Array(w.clone())));
            }
        }
        lines.push(match args.get("send_at").and_then(Value::as_str) {
            Some(at) => format!("Send time: {at}."),
            None => "Send time: now.".into(),
        });
    }
    lines.push(
        "Ask the person if this is right. Then call again with confirm: true to do it.".into(),
    );
    Ok(CallToolResult::success(vec![ContentBlock::text(
        lines.join("\n"),
    )]))
}

/// The request path: path parameters filled in and percent-encoded, then the query string.
fn uri(template: &str, args: &JsonObject, query: &[String]) -> Result<String, ErrorData> {
    let mut url = Url::parse("http://mcp.invalid/").map_err(|e| internal(e.to_string()))?;
    {
        let mut segs = url
            .path_segments_mut()
            .map_err(|()| internal("base URL cannot have a path"))?;
        segs.clear();
        for seg in template.trim_start_matches('/').split('/') {
            match seg.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                Some(name) => {
                    segs.push(&scalar(args.get(name)).ok_or_else(|| missing(name))?);
                }
                None => {
                    segs.push(seg);
                }
            }
        }
    }
    let pairs: Vec<(&str, String)> = query
        .iter()
        .filter_map(|q| scalar(args.get(q)).map(|v| (q.as_str(), v)))
        .collect();
    if !pairs.is_empty() {
        url.query_pairs_mut().extend_pairs(pairs);
    }
    Ok(match url.query() {
        Some(q) => format!("{}?{q}", url.path()),
        None => url.path().to_string(),
    })
}

/// A path or query value as text. Objects, arrays and null do not fit in a URL.
fn scalar(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn json_body(map: &ArgMap, args: &JsonObject) -> Option<Value> {
    if map.body_array {
        return Some(args.get(ITEMS_ARG).cloned().unwrap_or_else(|| json!([])));
    }
    if map.body_fields.is_empty() {
        return None;
    }
    Some(Value::Object(
        map.body_fields
            .iter()
            .filter_map(|f| args.get(f).map(|v| (f.clone(), v.clone())))
            .collect(),
    ))
}

/// `import_contacts`: the CSV text becomes the multipart `file` field the import route reads.
fn csv_upload(args: &JsonObject) -> Result<(String, Vec<u8>), ErrorData> {
    let csv = args
        .get("csv")
        .and_then(Value::as_str)
        .ok_or_else(|| missing("csv"))?;
    let consent = args.get("consent") == Some(&Value::Bool(true));
    let b = format!("mailrust-mcp-{}", Uuid::now_v7().simple());
    let mut body = format!(
        "--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"contacts.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n\
         --{b}\r\nContent-Disposition: form-data; name=\"consent\"\r\n\r\n{consent}\r\n"
    );
    if let Some(tag) = scalar(args.get("tag_id")) {
        body +=
            &format!("--{b}\r\nContent-Disposition: form-data; name=\"tag_id\"\r\n\r\n{tag}\r\n");
    }
    body += &format!("--{b}--\r\n");
    Ok((
        format!("multipart/form-data; boundary={b}"),
        body.into_bytes(),
    ))
}

fn request(
    method: &str,
    uri: String,
    body: Option<(String, Vec<u8>)>,
    auth: &HeaderValue,
) -> Result<Request<Body>, ErrorData> {
    let method = Method::from_bytes(method.to_ascii_uppercase().as_bytes())
        .map_err(|e| internal(e.to_string()))?;
    let b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, auth.clone());
    match body {
        Some((ct, bytes)) => b.header(header::CONTENT_TYPE, ct).body(Body::from(bytes)),
        None => b.body(Body::empty()),
    }
    .map_err(|e| internal(e.to_string()))
}

async fn send(api: &Router, req: Request<Body>) -> Result<Answer, ErrorData> {
    let res = api
        .clone()
        .oneshot(req)
        .await
        .unwrap_or_else(|never| match never {});
    let status = res.status();
    let content_type = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let bytes = axum::body::to_bytes(res.into_body(), MAX_ANSWER_BYTES)
        .await
        .map_err(|e| internal(format!("the answer could not be read: {e}")))?;
    Ok(Answer {
        status,
        content_type,
        bytes,
    })
}

/// 2xx JSON object → structured content; other 2xx → text; errors → `isError` with a next-step hint.
fn answer(def: &ToolDef, a: Answer) -> CallToolResult {
    if a.status.is_success() {
        if a.bytes.is_empty() {
            return CallToolResult::success(vec![ContentBlock::text("Done.")]);
        }
        if a.content_type.starts_with("application/json")
            && let Ok(v @ Value::Object(_)) = serde_json::from_slice::<Value>(&a.bytes)
        {
            return CallToolResult::structured(v);
        }
        return CallToolResult::success(vec![ContentBlock::text(
            String::from_utf8_lossy(&a.bytes).into_owned(),
        )]);
    }
    let body: Value = serde_json::from_slice(&a.bytes).unwrap_or(Value::Null);
    let code = body["error"]["code"].as_str().unwrap_or("error");
    let message = body["error"]["message"].as_str().unwrap_or("no message");
    CallToolResult::error(vec![ContentBlock::text(format!(
        "{} {code}: {message}\nHint: {}",
        a.status.as_u16(),
        hint(def, a.status, code)
    ))])
}

fn hint(def: &ToolDef, status: StatusCode, code: &str) -> String {
    match (status.as_u16(), code) {
        (401, _) => "The API key is missing, wrong or deleted. A person can make a new full-access key in the dashboard under API keys.".into(),
        (403, "forbidden") => "This is a sending key: it can only send emails and read them by id. Use a full-access key.".into(),
        (403, "account_paused") => "The account is paused, so nothing can be sent. A person must fix the cause and resume it in the dashboard.".into(),
        (404, _) => match list_tool(def.path) {
            Some(l) => format!("Nothing with that id. Call {l} to see the ids that exist."),
            None => "Nothing with that id.".into(),
        },
        (413, _) => "The input is too big. Send less at once.".into(),
        (429, _) => "Too many requests. Wait one second, then call again.".into(),
        (400..=499, _) => "Fix the input as the message says, then call again.".into(),
        _ => "The server failed. Try again later. If it keeps failing, a person should check the engine logs.".into(),
    }
}

/// The list tool for a path's first segment: `/contacts/{id}` → `list_contacts`.
fn list_tool(path: &str) -> Option<&'static str> {
    let first = path.trim_start_matches('/').split('/').next()?;
    TOOLS
        .iter()
        .find(|t| t.method == "get" && t.path.trim_start_matches('/') == first)
        .map(|t| t.name)
}

fn missing(name: &str) -> ErrorData {
    ErrorData::invalid_params(format!("missing or invalid argument `{name}`"), None)
}

fn internal(msg: impl Into<String>) -> ErrorData {
    ErrorData::internal_error(msg.into(), None)
}
