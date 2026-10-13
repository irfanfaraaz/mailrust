use axum::{
    Json, Router,
    extract::{
        DefaultBodyLimit, Multipart, Path, State,
        multipart::{MultipartError, MultipartRejection},
    },
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use uuid::Uuid;

use super::{AppState, auth::Auth, public::WebOnly};
use crate::{
    error::ApiError,
    images::{self, ImageError, MAX_IMAGE_BYTES},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // The 5 MB image plus multipart overhead; the global limit is far larger.
        .route(
            "/images",
            post(upload).layer(DefaultBodyLimit::max(MAX_IMAGE_BYTES + 64 * 1024)),
        )
        .route("/public/img/{id}", get(serve))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ImageUploaded {
    pub id: Uuid,
    /// Where readers load it: `{WEB_URL}/img/{id}`, whatever the store.
    pub url: String,
    pub width: u32,
    pub height: u32,
}

/// The multipart body of `POST /images` (documentation only).
#[allow(dead_code)]
#[derive(utoipa::ToSchema)]
pub struct ImageUpload {
    /// PNG, JPEG, GIF or WebP, ≤ 5 MB. The type is read from the bytes.
    #[schema(value_type = String, format = Binary)]
    pub file: Vec<u8>,
}

fn multipart_error(e: MultipartError) -> ApiError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::TooLarge
    } else {
        ApiError::Validation(e.body_text())
    }
}

pub(crate) async fn upload(
    State(state): State<AppState>,
    auth: Auth,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<(StatusCode, Json<ImageUploaded>), ApiError> {
    auth.require_full()?;
    let mut mp = multipart.map_err(|e| ApiError::Validation(e.body_text()))?;
    let mut file = None;
    while let Some(field) = mp.next_field().await.map_err(multipart_error)? {
        if field.name() == Some("file") {
            file = Some(field.bytes().await.map_err(multipart_error)?);
        }
    }
    let bytes =
        file.ok_or_else(|| ApiError::Validation("add the image in the `file` field".into()))?;
    let img = images::inspect(bytes.to_vec()).map_err(|e| match e {
        ImageError::TooLarge => ApiError::TooLarge,
        ImageError::Unsupported => ApiError::UnsupportedMedia(e.to_string()),
        ImageError::Broken => ApiError::Unprocessable("image_broken", e.to_string()),
    })?;
    let (width, height) = (img.width, img.height);
    let id = state
        .images
        .put(auth.account_id(), img)
        .await
        .map_err(anyhow::Error::from)?;
    tracing::info!(image_id = %id, account_id = %auth.account_id(), "image uploaded");
    Ok((
        StatusCode::CREATED,
        Json(ImageUploaded {
            id,
            url: format!("{}/img/{id}", state.config.web_base()),
            width,
            height,
        }),
    ))
}

pub(crate) async fn serve(
    State(state): State<AppState>,
    _: WebOnly,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let img = state
        .images
        .get(id)
        .await
        .map_err(anyhow::Error::from)?
        .ok_or(ApiError::NotFound)?;
    Ok((
        [
            (header::CONTENT_TYPE, img.content_type),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
            (
                header::CACHE_CONTROL,
                "public, max-age=31536000, immutable".to_string(),
            ),
        ],
        img.bytes,
    )
        .into_response())
}
