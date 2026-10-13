use axum::{
    Json, Router,
    extract::{
        Multipart, Path, State,
        multipart::{MultipartError, MultipartRejection},
    },
    http::StatusCode,
    routing::get,
};
use serde::Serialize;
use uuid::Uuid;

use super::{AppState, auth::Auth};
use crate::{
    error::ApiError,
    imports::{self, ImportView},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/contacts/imports", get(list).post(upload))
        .route("/contacts/imports/{id}", get(get_one))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ImportList {
    pub data: Vec<ImportView>,
}

/// The multipart body of `POST /contacts/imports` (documentation only).
#[allow(dead_code)]
#[derive(utoipa::ToSchema)]
pub struct ImportUpload {
    /// UTF-8 CSV, ≤ 20 MB, ≤ 100,000 rows, a header row with an `email` column.
    #[schema(value_type = String, format = Binary)]
    pub file: Vec<u8>,
    /// Must be `true`: these people agreed to get your emails.
    pub consent: bool,
    /// A tag added to every row.
    pub tag_id: Option<Uuid>,
}

fn multipart_error(e: MultipartError) -> ApiError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE { ApiError::TooLarge } else { ApiError::Validation(e.body_text()) }
}

pub(crate) async fn upload(
    State(state): State<AppState>,
    auth: Auth,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<(StatusCode, Json<ImportView>), ApiError> {
    auth.require_full()?;
    let mut mp = multipart.map_err(|e| ApiError::Validation(e.body_text()))?;
    let (mut file, mut consent, mut tag_id) = (None, false, None);
    while let Some(field) = mp.next_field().await.map_err(multipart_error)? {
        match field.name() {
            Some("file") => file = Some(field.bytes().await.map_err(multipart_error)?),
            Some("consent") => consent = field.text().await.map_err(multipart_error)?.trim() == "true",
            Some("tag_id") => {
                let t = field.text().await.map_err(multipart_error)?;
                if !t.trim().is_empty() {
                    tag_id = Some(t.trim().parse::<Uuid>().map_err(|_| ApiError::Validation("tag_id must be a tag id".into()))?);
                }
            }
            _ => {}
        }
    }
    if !consent {
        return Err(ApiError::Unprocessable("consent_required", "confirm that these people agreed to get your emails".into()));
    }
    let file = file.ok_or_else(|| ApiError::Validation("add the CSV file in the `file` field".into()))?;
    let rows_total = imports::check_file(&file).map_err(|(code, m)| ApiError::Unprocessable(code, m))?;
    let account_id = auth.account_id();
    if let Some(t) = tag_id {
        let mine = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM tags WHERE id = $1 AND account_id = $2) AS "e!""#,
            t,
            account_id
        )
        .fetch_one(&state.pool)
        .await?;
        if !mine {
            return Err(ApiError::Validation("unknown tag".into()));
        }
    }
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO contact_imports (id, account_id, created_by, file, tag_id, rows_total) VALUES ($1, $2, $3, $4, $5, $6)",
        id,
        account_id,
        auth.member.map(|m| m.user_id),
        &file[..],
        tag_id,
        rows_total
    )
    .execute(&state.pool)
    .await?;
    tracing::info!(import_id = %id, %account_id, rows_total, "import queued");
    let view = imports::view(&state.pool, account_id, id).await?.ok_or(ApiError::NotFound)?;
    Ok((StatusCode::CREATED, Json(view)))
}

pub(crate) async fn get_one(State(state): State<AppState>, auth: Auth, Path(id): Path<Uuid>) -> Result<Json<ImportView>, ApiError> {
    auth.require_full()?;
    imports::view(&state.pool, auth.account_id(), id).await?.map(Json).ok_or(ApiError::NotFound)
}

pub(crate) async fn list(State(state): State<AppState>, auth: Auth) -> Result<Json<ImportList>, ApiError> {
    auth.require_full()?;
    Ok(Json(ImportList { data: imports::list(&state.pool, auth.account_id()).await? }))
}
