use axum::{Json, Router, extract::State, routing::get};

use super::{AppState, auth::Auth};
use crate::{
    error::ApiError,
    stats::{self, Overview},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/stats/overview", get(overview))
}

pub(crate) async fn overview(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<Overview>, ApiError> {
    auth.require_full()?;
    stats::overview(&state.pool, auth.account_id())
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}
