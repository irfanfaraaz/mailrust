mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use mailrust::{
    keys::{self, KeyAccess},
    mcp::{
        self,
        tools::{LEFT_OUT, TOOLS},
    },
    openapi::ApiDoc,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use utoipa::OpenApi;

/// A JSON-RPC request to /mcp, the way a real client sends it.
fn rpc(key: Option<&str>, method: &str, params: Value) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri("/mcp")
        // rmcp reads Host even with the host check off; real clients always send it.
        .header("host", "localhost")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-06-18");
    if let Some(k) = key {
        b = b.header("authorization", format!("Bearer {k}"));
    }
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    b.body(Body::from(body.to_string())).unwrap()
}

async fn list_tools(t: &common::TestApp, key: &str) -> Vec<Value> {
    let (s, b) = common::call(&t.router, rpc(Some(key), "tools/list", json!({}))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    b["result"]["tools"].as_array().unwrap().clone()
}

/// tools/call. Returns the JSON-RPC `result`, or the whole body when it is a protocol error.
async fn tool(t: &common::TestApp, key: &str, name: &str, args: Value) -> Value {
    let params = json!({"name": name, "arguments": args});
    let (s, b) = common::call(&t.router, rpc(Some(key), "tools/call", params)).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    if b["result"].is_null() {
        b
    } else {
        b["result"].clone()
    }
}

fn text(result: &Value) -> String {
    result["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

async fn rest(
    t: &common::TestApp,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    common::call(&t.router, common::req(method, uri, Some(&t.key), body)).await
}

#[test]
fn every_api_route_is_a_tool_or_left_out_on_purpose() {
    let mut names = std::collections::HashSet::new();
    for t in TOOLS {
        assert!(names.insert(t.name), "duplicate tool {}", t.name);
    }
    let doc = serde_json::to_value(ApiDoc::openapi()).unwrap();
    for (path, item) in doc["paths"].as_object().unwrap() {
        if path.starts_with("/public/") {
            continue;
        }
        for method in item.as_object().unwrap().keys() {
            if !["get", "post", "put", "patch", "delete"].contains(&method.as_str()) {
                continue;
            }
            let is_tool = TOOLS.iter().any(|t| t.method == method && t.path == path);
            let left_out = LEFT_OUT.iter().any(|(m, p, _)| m == method && p == path);
            assert!(
                is_tool || left_out,
                "{method} {path} is neither an MCP tool nor in LEFT_OUT"
            );
            assert!(
                !(is_tool && left_out),
                "{method} {path} is both a tool and left out"
            );
        }
    }
    // Every row points at a real operation and its schema builds.
    assert_eq!(mcp::prepare().unwrap().len(), TOOLS.len());
}

#[test]
fn input_schemas_come_from_the_openapi_document() {
    let tools = mcp::prepare().unwrap();
    let schema = |name: &str| {
        let p = tools.iter().find(|p| p.def.name == name).unwrap();
        serde_json::to_value(&*p.tool.input_schema).unwrap()
    };
    let c = schema("create_contact");
    assert!(c["properties"]["email"].is_object(), "{c}");
    assert!(c["required"].as_array().unwrap().contains(&json!("email")));
    assert!(!c.to_string().contains("$ref"), "refs are inlined: {c}");
    let g = schema("get_contact");
    assert_eq!(g["required"], json!(["id"]));
    assert!(g["properties"].get("confirm").is_none());
    assert!(schema("delete_contact")["properties"]["confirm"].is_object());
    assert_eq!(
        schema("send_email_batch")["properties"]["items"]["type"],
        json!("array")
    );
    assert!(schema("list_contacts")["properties"]["limit"].is_object());
    let i = schema("import_contacts");
    assert!(i["properties"]["csv"].is_object());
    assert!(i["properties"].get("file").is_none());
}

#[sqlx::test]
async fn a_full_key_sees_every_tool_and_a_sending_key_only_email_tools(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    let all = list_tools(&t, &t.key).await;
    assert_eq!(all.len(), TOOLS.len());
    let by = |n: &str| all.iter().find(|t| t["name"] == n).unwrap().clone();
    assert_eq!(
        by("delete_contact")["annotations"]["destructiveHint"],
        json!(true)
    );
    assert_eq!(
        by("list_contacts")["annotations"]["readOnlyHint"],
        json!(true)
    );

    let sending = keys::create(&pool, t.account.id, "ai", KeyAccess::Sending)
        .await
        .unwrap()
        .token;
    let mut names: Vec<String> = list_tools(&t, &sending)
        .await
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    assert_eq!(names, ["get_email", "send_email", "send_email_batch"]);
}

#[sqlx::test]
async fn no_valid_api_key_is_a_401_before_any_mcp(pool: PgPool) {
    let t = common::app(pool).await;
    for key in [None, Some("mr_live_nope"), Some("mr_sess_whatever")] {
        let (s, _) = common::call(&t.router, rpc(key, "tools/list", json!({}))).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED, "{key:?}");
    }
}

#[sqlx::test]
async fn a_read_tool_returns_the_same_json_as_rest(pool: PgPool) {
    let t = common::app(pool).await;
    rest(&t, "POST", "/tags", Some(json!({"name": "vip"}))).await;
    let (_, via_rest) = rest(&t, "GET", "/tags", None).await;
    let r = tool(&t, &t.key, "list_tags", json!({})).await;
    assert_ne!(r["isError"], json!(true), "{r}");
    assert_eq!(r["structuredContent"], via_rest);
}

#[sqlx::test]
async fn a_destructive_tool_without_confirm_only_previews(pool: PgPool) {
    let t = common::app(pool).await;
    let (_, c) = rest(
        &t,
        "POST",
        "/contacts",
        Some(json!({"email": "ann@example.com"})),
    )
    .await;
    let id = c["id"].as_str().unwrap().to_string();
    let contact = format!("/contacts/{id}");

    let r = tool(&t, &t.key, "delete_contact", json!({"id": id})).await;
    assert_ne!(r["isError"], json!(true), "{r}");
    let msg = text(&r);
    assert!(msg.starts_with("Preview only"), "{msg}");
    assert!(
        msg.contains("ann@example.com"),
        "the preview shows the target: {msg}"
    );
    assert_eq!(rest(&t, "GET", &contact, None).await.0, StatusCode::OK);

    for not_yes in [json!(false), json!("true")] {
        let r = tool(
            &t,
            &t.key,
            "delete_contact",
            json!({"id": id, "confirm": not_yes}),
        )
        .await;
        assert!(text(&r).starts_with("Preview only"));
    }
    assert_eq!(rest(&t, "GET", &contact, None).await.0, StatusCode::OK);

    let r = tool(
        &t,
        &t.key,
        "delete_contact",
        json!({"id": id, "confirm": true}),
    )
    .await;
    assert_ne!(r["isError"], json!(true), "{r}");
    assert_eq!(text(&r), "Done.");
    assert_eq!(
        rest(&t, "GET", &contact, None).await.0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn send_campaign_preview_names_the_campaign_and_sends_nothing(pool: PgPool) {
    let t = common::app(pool).await;
    let (s, c) = rest(
        &t,
        "POST",
        "/campaigns",
        Some(json!({"subject": "Hello readers"})),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{c}");
    let id = c["id"].as_str().unwrap().to_string();
    let msg = text(&tool(&t, &t.key, "send_campaign", json!({"id": id})).await);
    assert!(msg.starts_with("Preview only"), "{msg}");
    assert!(msg.contains("Hello readers"), "{msg}");
    assert!(msg.contains("Send time: now"), "{msg}");
    let (_, after) = rest(&t, "GET", &format!("/campaigns/{id}"), None).await;
    assert_eq!(after["status"], json!("draft"));
}

#[sqlx::test]
async fn an_api_error_is_a_tool_error_with_a_hint(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    let r = tool(
        &t,
        &t.key,
        "get_contact",
        json!({"id": uuid::Uuid::now_v7()}),
    )
    .await;
    assert_eq!(r["isError"], json!(true));
    let msg = text(&r);
    assert!(
        msg.contains("not_found") && msg.contains("list_contacts"),
        "{msg}"
    );

    let sending = keys::create(&pool, t.account.id, "ai", KeyAccess::Sending)
        .await
        .unwrap()
        .token;
    let r = tool(&t, &sending, "list_emails", json!({})).await;
    assert_eq!(r["isError"], json!(true));
    assert!(text(&r).contains("sending key"), "{r}");
}

#[sqlx::test]
async fn unknown_tools_and_missing_arguments_are_protocol_errors(pool: PgPool) {
    let t = common::app(pool).await;
    let b = tool(&t, &t.key, "drop_database", json!({})).await;
    assert_eq!(b["error"]["code"], json!(-32602), "{b}");
    let b = tool(&t, &t.key, "get_contact", json!({})).await;
    assert_eq!(b["error"]["code"], json!(-32602), "{b}");
    assert!(
        b["error"]["message"].as_str().unwrap().contains("`id`"),
        "{b}"
    );
}

#[sqlx::test]
async fn import_contacts_takes_csv_text(pool: PgPool) {
    let t = common::app(pool).await;
    let csv = "email,first_name\nann@example.com,Ann\nbob@example.com,Bob\n";
    let r = tool(
        &t,
        &t.key,
        "import_contacts",
        json!({"csv": csv, "consent": true}),
    )
    .await;
    assert_ne!(r["isError"], json!(true), "{r}");
    let id = r["structuredContent"]["id"].as_str().unwrap().to_string();
    let g = tool(&t, &t.key, "get_import", json!({"id": id})).await;
    assert_eq!(g["structuredContent"]["id"], json!(id));

    let r = tool(
        &t,
        &t.key,
        "import_contacts",
        json!({"csv": "email\nx@example.com\n", "consent": false}),
    )
    .await;
    assert_eq!(r["isError"], json!(true));
    assert!(text(&r).contains("consent_required"), "{r}");
}

#[sqlx::test]
async fn path_values_are_encoded(pool: PgPool) {
    let t = common::app(pool).await;
    let (s, _) = rest(
        &t,
        "POST",
        "/suppressions",
        Some(json!({"address": "a+b@example.com"})),
    )
    .await;
    assert!(s.is_success());
    let r = tool(
        &t,
        &t.key,
        "remove_suppression",
        json!({"address": "a+b@example.com", "confirm": true}),
    )
    .await;
    assert_ne!(r["isError"], json!(true), "{r}");
    let (_, list) = rest(&t, "GET", "/suppressions", None).await;
    assert!(!list.to_string().contains("a+b@example.com"), "{list}");
}

#[sqlx::test]
async fn query_arguments_and_csv_answers_work(pool: PgPool) {
    let t = common::app(pool).await;
    for e in ["a@example.com", "b@example.com"] {
        rest(&t, "POST", "/contacts", Some(json!({"email": e}))).await;
    }
    let r = tool(&t, &t.key, "list_contacts", json!({"limit": 1})).await;
    assert_eq!(
        r["structuredContent"]["data"].as_array().unwrap().len(),
        1,
        "{r}"
    );
    let r = tool(&t, &t.key, "export_contacts", json!({})).await;
    assert_ne!(r["isError"], json!(true), "{r}");
    assert!(text(&r).contains("a@example.com"), "{r}");
}
