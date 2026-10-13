use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::Serialize;

use super::{AppState, auth::SessionAuth};
use crate::{
    error::ApiError,
    sessions,
    users::{self, MyAccount, User},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/auth/logout", post(logout))
        .route("/auth/logout-all", post(logout_all))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct MeResponse {
    pub user: User,
    pub accounts: Vec<MyAccount>,
}

pub(crate) async fn me(
    State(state): State<AppState>,
    SessionAuth(s): SessionAuth,
) -> Result<Json<MeResponse>, ApiError> {
    Ok(Json(MeResponse {
        accounts: users::accounts_of(&state.pool, s.user_id).await?,
        user: User {
            id: s.user_id,
            email: s.email,
        },
    }))
}

pub(crate) async fn logout(
    State(state): State<AppState>,
    SessionAuth(s): SessionAuth,
) -> Result<StatusCode, ApiError> {
    sessions::delete(&state.pool, s.session_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn logout_all(
    State(state): State<AppState>,
    SessionAuth(s): SessionAuth,
) -> Result<StatusCode, ApiError> {
    sessions::delete_all(&state.pool, s.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
