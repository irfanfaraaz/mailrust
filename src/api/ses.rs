use axum::{Router, extract::State, http::StatusCode, routing::post};

use super::AppState;
use crate::{
    error::ApiError,
    events::{self, SesEvent, sns::{self, SnsMessage}},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/ses/events", post(ses_events))
}

pub(crate) async fn ses_events(State(state): State<AppState>, body: String) -> Result<StatusCode, ApiError> {
    // Only exists when SES is configured.
    let Some(ses) = state.config.ses.as_ref() else { return Err(ApiError::NotFound) };
    let msg: SnsMessage = serde_json::from_str(&body).map_err(|_| ApiError::Validation("not an SNS message".into()))?;
    if msg.topic_arn != ses.sns_topic_arn {
        return Err(ApiError::Forbidden);
    }
    let key = state.sns_certs.key_for(&msg.signing_cert_url).await.map_err(|e| {
        tracing::warn!(error = %e, "SNS certificate rejected");
        ApiError::Forbidden
    })?;
    sns::verify_with_key(&msg, &key).map_err(|e| {
        tracing::warn!(error = %e, "SNS signature rejected");
        ApiError::Forbidden
    })?;

    match msg.kind.as_str() {
        "SubscriptionConfirmation" => {
            let url = msg.subscribe_url.as_deref().filter(|u| sns::is_sns_host_url(u)).ok_or(ApiError::Forbidden)?;
            state
                .sns_certs
                .client()
                .get(url)
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .map_err(|e| ApiError::Internal(e.into()))?;
            tracing::info!(topic = %msg.topic_arn, "SNS subscription confirmed");
        }
        "Notification" => {
            let ev: SesEvent = serde_json::from_str(&msg.message)
                .map_err(|e| ApiError::Validation(format!("not an SES event: {e}")))?;
            events::ingest(&state.pool, &ev, Some(&msg.message_id)).await?;
        }
        _ => {}
    }
    Ok(StatusCode::OK)
}
