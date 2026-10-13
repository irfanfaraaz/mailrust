#![allow(dead_code)] // the documented fns below exist only for their #[utoipa::path] attributes

use axum::{Json, Router, routing::get};
use serde::Serialize;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::{
    accounts::AccountStatus,
    api::{
        AppState,
        domains::{CreateDomainRequest, DomainList, UpdateDomainRequest},
        emails::{BatchResponse, EmailList},
        keys::{CreateKeyRequest, CreatedKey, KeyList},
        login::{LoginRequest, SessionResponse, TokenRequest},
        me::MeResponse,
        suppressions::{AddSuppressionRequest, SuppressionList, SuppressionView},
        webhooks::{CreateWebhookRequest, WebhookList},
    },
    domains::{Domain, DomainStatus},
    emails::{
        AttachmentInput, EmailStatus, EmailSummary, EmailView, EventView, Priority,
        SendEmailRequest, SendResponse,
    },
    keys::{KeyAccess, KeySummary},
    safety::SuppressionReason,
    sender::DnsRecord,
    users::{MemberRole, MyAccount, User},
    webhooks::WebhookView,
};

#[derive(Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ErrorDetail {
    /// Stable machine code, e.g. `validation_error`, `account_paused`, `domain_not_verified`.
    pub code: String,
    pub message: String,
}

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("mr_live_… or mr_sess_…")
                    .build(),
            ),
        );
    }
}

#[utoipa::path(post, path = "/emails", tag = "emails", request_body = SendEmailRequest,
    params(("Idempotency-Key" = Option<String>, Header, description = "Same key + same body within 24 h returns the first email id")),
    responses((status = 200, body = SendResponse), (status = 400, body = ErrorBody), (status = 403, body = ErrorBody),
               (status = 409, body = ErrorBody), (status = 413, body = ErrorBody), (status = 422, body = ErrorBody), (status = 429, body = ErrorBody)))]
fn send_email() {}

#[utoipa::path(post, path = "/emails/batch", tag = "emails", request_body = Vec<SendEmailRequest>,
    responses((status = 200, body = BatchResponse), (status = 400, body = ErrorBody), (status = 422, body = ErrorBody)))]
fn send_batch() {}

#[utoipa::path(get, path = "/emails", tag = "emails",
    params(("status" = Option<EmailStatus>, Query), ("limit" = Option<i64>, Query), ("after" = Option<uuid::Uuid>, Query)),
    responses((status = 200, body = EmailList)))]
fn list_emails() {}

#[utoipa::path(get, path = "/emails/{id}", tag = "emails", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = EmailView), (status = 404, body = ErrorBody)))]
fn get_email() {}

#[utoipa::path(post, path = "/emails/{id}/cancel", tag = "emails", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = SendResponse), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
fn cancel_email() {}

#[utoipa::path(post, path = "/domains", tag = "domains", request_body = CreateDomainRequest,
    responses((status = 201, body = Domain), (status = 400, body = ErrorBody), (status = 409, body = ErrorBody)))]
fn create_domain() {}

#[utoipa::path(get, path = "/domains", tag = "domains", responses((status = 200, body = DomainList)))]
fn list_domains() {}

#[utoipa::path(get, path = "/domains/{id}", tag = "domains", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = Domain), (status = 404, body = ErrorBody)))]
fn get_domain() {}

#[utoipa::path(patch, path = "/domains/{id}", tag = "domains", params(("id" = uuid::Uuid, Path)), request_body = UpdateDomainRequest,
    responses((status = 200, body = Domain), (status = 404, body = ErrorBody)))]
fn update_domain() {}

#[utoipa::path(delete, path = "/domains/{id}", tag = "domains", params(("id" = uuid::Uuid, Path)),
    responses((status = 204), (status = 404, body = ErrorBody)))]
fn delete_domain() {}

#[utoipa::path(post, path = "/domains/{id}/verify", tag = "domains", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = Domain), (status = 404, body = ErrorBody)))]
fn verify_domain() {}

#[utoipa::path(post, path = "/api-keys", tag = "api keys", request_body = CreateKeyRequest, responses((status = 201, body = CreatedKey)))]
fn create_key() {}

#[utoipa::path(get, path = "/api-keys", tag = "api keys", responses((status = 200, body = KeyList)))]
fn list_keys() {}

#[utoipa::path(delete, path = "/api-keys/{id}", tag = "api keys", params(("id" = uuid::Uuid, Path)),
    responses((status = 204), (status = 404, body = ErrorBody)))]
fn delete_key() {}

#[utoipa::path(post, path = "/webhooks", tag = "webhooks", request_body = CreateWebhookRequest,
    responses((status = 201, body = WebhookView), (status = 400, body = ErrorBody)))]
fn create_webhook() {}

#[utoipa::path(get, path = "/webhooks", tag = "webhooks", responses((status = 200, body = WebhookList)))]
fn list_webhooks() {}

#[utoipa::path(delete, path = "/webhooks/{id}", tag = "webhooks", params(("id" = uuid::Uuid, Path)),
    responses((status = 204), (status = 404, body = ErrorBody)))]
fn delete_webhook() {}

#[utoipa::path(get, path = "/suppressions", tag = "suppressions",
    params(("limit" = Option<i64>, Query), ("after" = Option<String>, Query)), responses((status = 200, body = SuppressionList)))]
fn list_suppressions() {}

#[utoipa::path(post, path = "/suppressions", tag = "suppressions", request_body = AddSuppressionRequest,
    responses((status = 201, body = SuppressionView), (status = 400, body = ErrorBody)))]
fn add_suppression() {}

#[utoipa::path(delete, path = "/suppressions/{address}", tag = "suppressions", params(("address" = String, Path)),
    responses((status = 204), (status = 404, body = ErrorBody)))]
fn delete_suppression() {}

#[utoipa::path(get, path = "/health", tag = "system", security(), responses((status = 200), (status = 503)))]
fn health() {}

#[utoipa::path(post, path = "/auth/login", tag = "dashboard", security(), request_body = LoginRequest,
    responses((status = 202), (status = 400, body = ErrorBody)))]
fn auth_login() {}

#[utoipa::path(post, path = "/auth/verify", tag = "dashboard", security(), request_body = TokenRequest,
    responses((status = 200, body = SessionResponse), (status = 401, body = ErrorBody)))]
fn auth_verify() {}

#[utoipa::path(get, path = "/me", tag = "dashboard",
    responses((status = 200, body = MeResponse), (status = 401, body = ErrorBody)))]
fn me() {}

#[utoipa::path(post, path = "/auth/logout", tag = "dashboard", responses((status = 204), (status = 401, body = ErrorBody)))]
fn auth_logout() {}

#[utoipa::path(post, path = "/auth/logout-all", tag = "dashboard", responses((status = 204), (status = 401, body = ErrorBody)))]
fn auth_logout_all() {}

#[derive(OpenApi)]
#[openapi(
    info(title = "Mailrust API", version = "0.1.0", description = "Send transactional email through your own Amazon SES. Auth: `Authorization: Bearer mr_live_…` (API key), or `Bearer mr_sess_…` (dashboard session) plus `X-Mailrust-Account: <account id>`."),
    paths(send_email, send_batch, list_emails, get_email, cancel_email, create_domain, list_domains, get_domain, update_domain,
          delete_domain, verify_domain, create_key, list_keys, delete_key, create_webhook, list_webhooks, delete_webhook,
          list_suppressions, add_suppression, delete_suppression, health, auth_login, auth_verify, me, auth_logout, auth_logout_all),
    components(schemas(SendEmailRequest, AttachmentInput, SendResponse, BatchResponse, EmailView, EmailSummary, EmailList,
        EventView, EmailStatus, Priority, Domain, DomainStatus, DnsRecord, DomainList, CreateDomainRequest, UpdateDomainRequest,
        KeySummary, KeyAccess, CreateKeyRequest, CreatedKey, KeyList, CreateWebhookRequest, WebhookView, WebhookList,
        SuppressionView, SuppressionList, AddSuppressionRequest, SuppressionReason, AccountStatus, LoginRequest, TokenRequest, SessionResponse,
        MeResponse, MyAccount, User, MemberRole, ErrorBody, ErrorDetail)),
    modifiers(&BearerAuth),
    security(("bearer" = []))
)]
pub struct ApiDoc;

pub fn routes() -> Router<AppState> {
    Router::new().route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
}
