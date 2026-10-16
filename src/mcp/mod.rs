//! `POST /mcp`: the Model Context Protocol endpoint. An AI client lists the tools and calls them;
//! each call runs through the REST router in memory, as the caller's API key.

pub mod call;
pub mod schema;
pub mod tools;

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{header::AUTHORIZATION, request::Parts},
    response::{IntoResponse, Response},
    routing::post,
};
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, Implementation, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
    },
    service::RequestContext,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
    },
};
use tower::ServiceExt;
use utoipa::OpenApi;

use crate::{
    api::AppState,
    error::ApiError,
    keys::{self, KeyAccess},
};
use schema::ArgMap;
use tools::{Kind, ToolDef};

const INSTRUCTIONS: &str = "Mailrust is a self-hosted email platform: transactional email through the account's own \
Amazon SES, and a newsletter (contacts, tags, campaigns). Tools that cannot be undone need `confirm: true`; \
call them without it first, show the person the preview, and only then confirm.";

/// One tool, ready to list and call.
pub struct Prepared {
    pub def: &'static ToolDef,
    pub tool: Tool,
    pub args: ArgMap,
}

/// Builds every tool from the table and the OpenAPI document. Errors on drift (tests/mcp.rs checks this).
pub fn prepare() -> Result<Vec<Prepared>, String> {
    let doc = serde_json::to_value(crate::openapi::ApiDoc::openapi()).map_err(|e| e.to_string())?;
    tools::TOOLS
        .iter()
        .map(|def| {
            let (input, args) = schema::build(&doc, def)?;
            let hints = ToolAnnotations::with_title(def.title)
                .read_only(def.kind == Kind::Read)
                .destructive(def.kind == Kind::Destructive)
                .idempotent(def.kind == Kind::Read);
            let tool = Tool::new(def.name, def.description, input)
                .with_title(def.title)
                .with_annotations(hints);
            Ok(Prepared { def, tool, args })
        })
        .collect()
}

#[derive(Clone)]
struct Mailrust {
    api: Router,
    tools: Arc<Vec<Prepared>>,
}

impl ServerHandler for Mailrust {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("mailrust", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let sending = access(&ctx) == Some(KeyAccess::Sending);
        Ok(ListToolsResult::with_all_items(
            self.tools
                .iter()
                .filter(|p| !sending || p.def.sending)
                .map(|p| p.tool.clone())
                .collect(),
        ))
    }

    async fn call_tool(
        &self,
        req: CallToolRequestParams,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let p = self
            .tools
            .iter()
            .find(|p| p.def.name == req.name)
            .ok_or_else(|| {
                ErrorData::invalid_params(
                    format!("unknown tool `{}`; call tools/list", req.name),
                    None,
                )
            })?;
        let auth = parts(&ctx)
            .and_then(|parts| parts.headers.get(AUTHORIZATION))
            .cloned()
            .ok_or_else(|| {
                ErrorData::internal_error("the request lost its Authorization header", None)
            })?;
        call::run(
            &self.api,
            &auth,
            p.def,
            &p.args,
            req.arguments.unwrap_or_default(),
        )
        .await
        .map(Into::into)
    }
}

/// rmcp puts the HTTP request parts in the context; `handle` put the key's access in their extensions.
fn parts(ctx: &RequestContext<RoleServer>) -> Option<&Parts> {
    ctx.extensions.get::<Parts>()
}

fn access(ctx: &RequestContext<RoleServer>) -> Option<KeyAccess> {
    parts(ctx)?.extensions.get::<KeyAccess>().copied()
}

#[derive(Clone)]
struct Endpoint {
    state: AppState,
    mcp: StreamableHttpService<Mailrust, NeverSessionManager>,
}

/// Adds `POST /mcp` to the finished API router. The tools call that same router, without `/mcp`.
pub fn mount(api: Router, state: AppState) -> Router {
    let tools =
        prepare().unwrap_or_else(|e| panic!("the MCP tool table does not match the API: {e}"));
    let server = Mailrust {
        api: api.clone(),
        tools: Arc::new(tools),
    };
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        // Every request needs a bearer key, so the DNS-rebinding guard (loopback hosts only) adds nothing,
        // and it would block the engine's public host and custom domains.
        .disable_allowed_hosts();
    let mcp = StreamableHttpService::new(
        move || Ok(server.clone()),
        Arc::new(NeverSessionManager::default()),
        config,
    );
    api.route("/mcp", post(handle).with_state(Endpoint { state, mcp }))
}

/// Checks the API key, then hands the request to MCP. No valid key: 401 with no MCP body.
async fn handle(State(e): State<Endpoint>, mut req: Request) -> Response {
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .unwrap_or_default();
    if !token.starts_with(keys::TOKEN_PREFIX) {
        return ApiError::Unauthorized.into_response();
    }
    let access = match keys::find_by_token(&e.state.pool, token).await {
        Ok(Some(ctx)) => ctx.access,
        Ok(None) => return ApiError::Unauthorized.into_response(),
        Err(err) => return ApiError::from(err).into_response(),
    };
    req.extensions_mut().insert(access);
    let res = e
        .mcp
        .oneshot(req)
        .await
        .unwrap_or_else(|never| match never {});
    res.map(Body::new)
}
