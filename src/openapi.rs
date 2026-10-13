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
        campaigns::{
            CampaignList, CampaignPreview, CampaignRequest, CampaignSummary, SendRequest,
            TestSendRequest, TestSendResult,
        },
        contacts::{ContactList, CreateContactRequest, UpdateContactRequest},
        domains::{CreateDomainRequest, DomainList, UpdateDomainRequest},
        emails::{BatchResponse, EmailList},
        images::{ImageUpload, ImageUploaded},
        imports::{ImportList, ImportUpload},
        keys::{CreateKeyRequest, CreatedKey, KeyList},
        login::{LoginRequest, SessionResponse, TokenRequest},
        me::MeResponse,
        newsletter::UpdateNewsletterRequest,
        public::{
            ArchiveIndex, ArchiveIssue, ConfirmInfo, ConfirmView, PublicJoinRequest,
            PublicNewsletter, UnsubscribeView,
        },
        suppressions::{AddSuppressionRequest, SuppressionList, SuppressionView},
        tags::{TagList, TagRequest, TagView},
        team::{InviteRequest, RoleRequest},
        webhooks::{CreateWebhookRequest, WebhookList},
    },
    campaigns::{
        Campaign, CampaignStatus,
        checks::Check,
        report::{CampaignReport, LinkStat, ProviderStat, ReportRates, ReportTotals, WaveStat},
    },
    contacts::{ContactSource, ContactStatus, ContactView, TagRef},
    domains::{Domain, DomainStatus},
    emails::{
        AttachmentInput, EmailStatus, EmailSummary, EmailView, EventView, Priority,
        SendEmailRequest, SendResponse,
    },
    imports::{ImportRowError, ImportStatus, ImportView},
    keys::{KeyAccess, KeySummary},
    newsletter::NewsletterSettings,
    safety::SuppressionReason,
    sender::DnsRecord,
    stats::{Counts, Overview},
    team::{InviteView, MemberView, TeamView},
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
    params(("status" = Option<EmailStatus>, Query), ("priority" = Option<Priority>, Query),
        ("since" = Option<chrono::DateTime<chrono::Utc>>, Query), ("until" = Option<chrono::DateTime<chrono::Utc>>, Query),
        ("q" = Option<String>, Query), ("limit" = Option<i64>, Query), ("after" = Option<uuid::Uuid>, Query)),
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
    params(("limit" = Option<i64>, Query), ("after" = Option<String>, Query), ("q" = Option<String>, Query)),
    responses((status = 200, body = SuppressionList)))]
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

#[utoipa::path(get, path = "/team", tag = "dashboard",
    responses((status = 200, body = TeamView), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
fn team_list() {}

#[utoipa::path(post, path = "/team/invites", tag = "dashboard", request_body = InviteRequest,
    responses((status = 201, body = InviteView), (status = 400, body = ErrorBody), (status = 403, body = ErrorBody), (status = 409, body = ErrorBody)))]
fn team_invite() {}

#[utoipa::path(delete, path = "/team/invites/{id}", tag = "dashboard", params(("id" = uuid::Uuid, Path)),
    responses((status = 204), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
fn team_cancel_invite() {}

#[utoipa::path(patch, path = "/team/members/{user_id}", tag = "dashboard", params(("user_id" = uuid::Uuid, Path)), request_body = RoleRequest,
    responses((status = 200, body = MemberView), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
fn team_set_role() {}

#[utoipa::path(delete, path = "/team/members/{user_id}", tag = "dashboard", params(("user_id" = uuid::Uuid, Path)),
    responses((status = 204), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
fn team_remove() {}

#[utoipa::path(post, path = "/invites/accept", tag = "dashboard", security(), request_body = TokenRequest,
    responses((status = 200, body = SessionResponse), (status = 401, body = ErrorBody)))]
fn invites_accept() {}

#[utoipa::path(get, path = "/stats/overview", tag = "stats", responses((status = 200, body = Overview)))]
fn stats_overview() {}

#[utoipa::path(get, path = "/contacts", tag = "audience",
    params(("status" = Option<ContactStatus>, Query), ("tag" = Option<uuid::Uuid>, Query, description = "Tag id"),
        ("q" = Option<String>, Query, description = "Email or name, case-insensitive"),
        ("limit" = Option<i64>, Query), ("after" = Option<uuid::Uuid>, Query)),
    responses((status = 200, body = ContactList)))]
fn list_contacts() {}

#[utoipa::path(post, path = "/contacts", tag = "audience", request_body = CreateContactRequest,
    responses((status = 201, body = ContactView), (status = 200, body = ContactView, description = "The address existed: updated"),
               (status = 400, body = ErrorBody)))]
fn create_contact() {}

#[utoipa::path(get, path = "/contacts/export", tag = "audience",
    responses((status = 200, content_type = "text/csv", body = String,
               description = "email,first_name,last_name,status,tags,created_at, then one column per field name")))]
fn export_contacts() {}

#[utoipa::path(get, path = "/contacts/{id}", tag = "audience", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = ContactView), (status = 404, body = ErrorBody)))]
fn get_contact() {}

#[utoipa::path(patch, path = "/contacts/{id}", tag = "audience", params(("id" = uuid::Uuid, Path)), request_body = UpdateContactRequest,
    responses((status = 200, body = ContactView), (status = 400, body = ErrorBody), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
fn update_contact() {}

#[utoipa::path(delete, path = "/contacts/{id}", tag = "audience", params(("id" = uuid::Uuid, Path)),
    responses((status = 204), (status = 404, body = ErrorBody)))]
fn delete_contact() {}

#[utoipa::path(get, path = "/newsletter", tag = "audience", responses((status = 200, body = NewsletterSettings)))]
fn get_newsletter() {}

#[utoipa::path(patch, path = "/newsletter", tag = "audience", request_body = UpdateNewsletterRequest,
    responses((status = 200, body = NewsletterSettings), (status = 400, body = ErrorBody),
               (status = 409, body = ErrorBody), (status = 422, body = ErrorBody)))]
fn update_newsletter() {}

#[utoipa::path(get, path = "/tags", tag = "audience", responses((status = 200, body = TagList)))]
fn list_tags() {}

#[utoipa::path(post, path = "/tags", tag = "audience", request_body = TagRequest,
    responses((status = 201, body = TagView), (status = 400, body = ErrorBody), (status = 409, body = ErrorBody)))]
fn create_tag() {}

#[utoipa::path(patch, path = "/tags/{id}", tag = "audience", params(("id" = uuid::Uuid, Path)), request_body = TagRequest,
    responses((status = 200, body = TagView), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
fn rename_tag() {}

#[utoipa::path(delete, path = "/tags/{id}", tag = "audience", params(("id" = uuid::Uuid, Path)),
    responses((status = 204), (status = 404, body = ErrorBody)))]
fn delete_tag() {}

#[utoipa::path(get, path = "/public/newsletter/{slug}", tag = "public", security(),
    params(("slug" = String, Path), ("X-Mailrust-Web" = String, Header)),
    responses((status = 200, body = PublicNewsletter), (status = 404, body = ErrorBody), (status = 429, body = ErrorBody)))]
fn public_newsletter() {}

#[utoipa::path(get, path = "/public/unsubscribe/{token}", tag = "public", security(),
    params(("token" = String, Path), ("X-Mailrust-Web" = String, Header)),
    responses((status = 200, body = UnsubscribeView)))]
fn public_unsubscribe_info() {}

#[utoipa::path(post, path = "/public/unsubscribe", tag = "public", security(), request_body = TokenRequest,
    params(("X-Mailrust-Web" = String, Header)), responses((status = 200, body = UnsubscribeView)))]
fn public_unsubscribe() {}

#[utoipa::path(post, path = "/public/resubscribe", tag = "public", security(), request_body = TokenRequest,
    params(("X-Mailrust-Web" = String, Header)), responses((status = 204), (status = 409, body = ErrorBody)))]
fn public_resubscribe() {}

#[utoipa::path(post, path = "/public/join", tag = "public", security(), request_body = PublicJoinRequest,
    params(("X-Mailrust-Web" = String, Header)),
    responses((status = 202, description = "Same answer for every address"), (status = 400, body = ErrorBody),
               (status = 404, body = ErrorBody), (status = 409, body = ErrorBody), (status = 429, body = ErrorBody)))]
fn public_join() {}

#[utoipa::path(post, path = "/public/confirm", tag = "public", security(), request_body = TokenRequest,
    params(("X-Mailrust-Web" = String, Header)),
    responses((status = 200, body = ConfirmView), (status = 401, body = ErrorBody)))]
fn public_confirm() {}

#[utoipa::path(get, path = "/public/confirm/{token}", tag = "public", security(),
    params(("token" = String, Path), ("X-Mailrust-Web" = String, Header)),
    responses((status = 200, body = ConfirmInfo)))]
fn public_confirm_info() {}

#[utoipa::path(get, path = "/public/archive/{slug}", tag = "public", security(),
    params(("slug" = String, Path), ("X-Mailrust-Web" = String, Header)),
    responses((status = 200, body = ArchiveIndex), (status = 404, body = ErrorBody), (status = 429, body = ErrorBody)))]
fn public_archive() {}

#[utoipa::path(get, path = "/public/archive/{slug}/{issue}", tag = "public", security(),
    params(("slug" = String, Path), ("issue" = String, Path), ("X-Mailrust-Web" = String, Header)),
    responses((status = 200, content_type = "text/html", body = String, description = "The archive page, served with a strict CSP"),
               (status = 404, body = ErrorBody)))]
fn public_archive_issue() {}

#[utoipa::path(post, path = "/contacts/imports", tag = "audience",
    request_body(content = ImportUpload, content_type = "multipart/form-data"),
    responses((status = 201, body = ImportView), (status = 400, body = ErrorBody), (status = 413, body = ErrorBody),
               (status = 422, body = ErrorBody, description = "consent_required, file_too_large, not_utf8, bad_csv, empty_file, no_email_column, too_many_rows")))]
fn upload_import() {}

#[utoipa::path(get, path = "/contacts/imports", tag = "audience", responses((status = 200, body = ImportList)))]
fn list_imports() {}

#[utoipa::path(get, path = "/contacts/imports/{id}", tag = "audience", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = ImportView), (status = 404, body = ErrorBody)))]
fn get_import() {}

#[utoipa::path(post, path = "/images", tag = "campaigns",
    request_body(content = ImageUpload, content_type = "multipart/form-data"),
    responses((status = 201, body = ImageUploaded), (status = 400, body = ErrorBody), (status = 413, body = ErrorBody),
               (status = 415, body = ErrorBody), (status = 422, body = ErrorBody)))]
fn upload_image() {}

#[utoipa::path(get, path = "/public/img/{id}", tag = "public", security(),
    params(("id" = uuid::Uuid, Path), ("X-Mailrust-Web" = String, Header)),
    responses((status = 200, content_type = "image/*", body = String, description = "The image bytes"), (status = 404, body = ErrorBody)))]
fn public_image() {}

#[utoipa::path(get, path = "/campaigns", tag = "campaigns",
    params(("limit" = Option<i64>, Query), ("after" = Option<uuid::Uuid>, Query)),
    responses((status = 200, body = CampaignList)))]
fn list_campaigns() {}

#[utoipa::path(post, path = "/campaigns", tag = "campaigns", request_body = CampaignRequest,
    responses((status = 201, body = Campaign), (status = 400, body = ErrorBody), (status = 422, body = ErrorBody)))]
fn create_campaign() {}

#[utoipa::path(get, path = "/campaigns/{id}", tag = "campaigns", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = Campaign), (status = 404, body = ErrorBody)))]
fn get_campaign() {}

#[utoipa::path(patch, path = "/campaigns/{id}", tag = "campaigns", params(("id" = uuid::Uuid, Path)), request_body = CampaignRequest,
    responses((status = 200, body = Campaign), (status = 400, body = ErrorBody), (status = 404, body = ErrorBody),
               (status = 409, body = ErrorBody), (status = 422, body = ErrorBody)))]
fn update_campaign() {}

#[utoipa::path(delete, path = "/campaigns/{id}", tag = "campaigns", params(("id" = uuid::Uuid, Path)),
    responses((status = 204), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
fn delete_campaign() {}

#[utoipa::path(post, path = "/campaigns/{id}/preview", tag = "campaigns", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = CampaignPreview), (status = 404, body = ErrorBody)))]
fn preview_campaign() {}

#[utoipa::path(post, path = "/campaigns/{id}/test", tag = "campaigns", params(("id" = uuid::Uuid, Path)), request_body = TestSendRequest,
    responses((status = 202, body = TestSendResult), (status = 400, body = ErrorBody), (status = 404, body = ErrorBody),
               (status = 422, body = ErrorBody), (status = 429, body = ErrorBody)))]
fn test_campaign() {}

#[utoipa::path(post, path = "/campaigns/{id}/send", tag = "campaigns", params(("id" = uuid::Uuid, Path)), request_body = SendRequest,
    responses((status = 200, body = Campaign), (status = 400, body = ErrorBody), (status = 404, body = ErrorBody),
               (status = 409, body = ErrorBody), (status = 422, body = ErrorBody, description = "Named by the first failed check")))]
fn send_campaign() {}

#[utoipa::path(get, path = "/campaigns/{id}/report", tag = "campaigns", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = CampaignReport), (status = 404, body = ErrorBody)))]
fn campaign_report() {}

#[utoipa::path(post, path = "/campaigns/{id}/pause", tag = "campaigns", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = Campaign), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
fn pause_campaign() {}

#[utoipa::path(post, path = "/campaigns/{id}/resume", tag = "campaigns", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = Campaign), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody), (status = 422, body = ErrorBody)))]
fn resume_campaign() {}

#[utoipa::path(post, path = "/campaigns/{id}/cancel", tag = "campaigns", params(("id" = uuid::Uuid, Path)),
    responses((status = 200, body = Campaign), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
fn cancel_campaign() {}

#[derive(OpenApi)]
#[openapi(
    info(title = "Mailrust API", version = "0.1.0", description = "Send transactional email through your own Amazon SES. Auth: `Authorization: Bearer mr_live_…` (API key), or `Bearer mr_sess_…` (dashboard session) plus `X-Mailrust-Account: <account id>`."),
    paths(send_email, send_batch, list_emails, get_email, cancel_email, create_domain, list_domains, get_domain, update_domain,
          delete_domain, verify_domain, create_key, list_keys, delete_key, create_webhook, list_webhooks, delete_webhook,
          list_suppressions, add_suppression, delete_suppression, health, auth_login, auth_verify, me, auth_logout, auth_logout_all,
          team_list, team_invite, team_cancel_invite, team_set_role, team_remove, invites_accept, stats_overview,
          list_contacts, export_contacts, create_contact, get_contact, update_contact, delete_contact,
          list_tags, create_tag, rename_tag, delete_tag, get_newsletter, update_newsletter,
          public_newsletter, public_unsubscribe_info, public_unsubscribe, public_resubscribe,
          public_join, public_confirm, public_confirm_info, public_archive, public_archive_issue, upload_import, list_imports, get_import,
          upload_image, public_image,
          list_campaigns, create_campaign, get_campaign, update_campaign, delete_campaign, preview_campaign, test_campaign, send_campaign,
          campaign_report, pause_campaign, resume_campaign, cancel_campaign),
    components(schemas(SendEmailRequest, AttachmentInput, SendResponse, BatchResponse, EmailView, EmailSummary, EmailList,
        EventView, EmailStatus, Priority, Domain, DomainStatus, DnsRecord, DomainList, CreateDomainRequest, UpdateDomainRequest,
        KeySummary, KeyAccess, CreateKeyRequest, CreatedKey, KeyList, CreateWebhookRequest, WebhookView, WebhookList,
        SuppressionView, SuppressionList, AddSuppressionRequest, SuppressionReason, AccountStatus, LoginRequest, TokenRequest, SessionResponse,
        MeResponse, MyAccount, User, MemberRole, TeamView, MemberView, InviteView, InviteRequest, RoleRequest, Overview, Counts, ErrorBody, ErrorDetail,
        ContactView, ContactList, ContactStatus, ContactSource, TagRef, CreateContactRequest, UpdateContactRequest,
        TagView, TagList, TagRequest, NewsletterSettings, UpdateNewsletterRequest,
        PublicNewsletter, UnsubscribeView, PublicJoinRequest, ConfirmView, ConfirmInfo, ArchiveIndex, ArchiveIssue,
        ImportView, ImportList, ImportStatus, ImportRowError, ImportUpload, ImageUpload, ImageUploaded,
        Campaign, CampaignStatus, CampaignRequest, CampaignSummary, CampaignList, CampaignPreview, Check,
        CampaignReport, ReportTotals, ReportRates, LinkStat, ProviderStat, WaveStat,
        TestSendRequest, TestSendResult, SendRequest)),
    modifiers(&BearerAuth),
    security(("bearer" = []))
)]
pub struct ApiDoc;

pub fn routes() -> Router<AppState> {
    Router::new().route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
}

#[cfg(test)]
mod tests {
    use utoipa::OpenApi;

    #[test]
    fn the_document_lists_the_dashboard_routes() {
        let doc = serde_json::to_value(super::ApiDoc::openapi()).unwrap();
        for p in [
            "/auth/login",
            "/auth/verify",
            "/me",
            "/team",
            "/team/invites",
            "/invites/accept",
            "/stats/overview",
        ] {
            assert!(doc["paths"][p].is_object(), "missing {p}");
        }
    }

    #[test]
    fn the_document_lists_the_audience_routes() {
        let doc = serde_json::to_value(super::ApiDoc::openapi()).unwrap();
        for p in [
            "/contacts",
            "/contacts/export",
            "/contacts/{id}",
            "/tags",
            "/tags/{id}",
            "/newsletter",
            "/public/newsletter/{slug}",
            "/public/unsubscribe/{token}",
            "/public/unsubscribe",
            "/public/resubscribe",
            "/public/join",
            "/public/confirm",
            "/contacts/imports",
            "/contacts/imports/{id}",
        ] {
            assert!(doc["paths"][p].is_object(), "missing {p}");
        }
    }
}
