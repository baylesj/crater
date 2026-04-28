//! Settings API endpoints.
//!
//! POST /api/settings/sc-token      — save a manually-entered SC OAuth token
//! GET  /api/settings/sc-token/test — verify the stored token against SC's /me

use axum::{extract::State, Json};

use crate::{error::{ApiResult, AppError}, state::SharedState};

#[derive(serde::Deserialize)]
pub struct SaveTokenRequest {
    pub token: String,
}

/// Store a manually entered SoundCloud OAuth token in the kv table.
/// Strips a leading "OAuth " prefix if present (copied verbatim from DevTools).
pub async fn save_sc_token(
    State(state): State<SharedState>,
    Json(req): Json<SaveTokenRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let token = req.token.trim().trim_start_matches("OAuth ").to_owned();
    if token.is_empty() {
        return Err(AppError::BadRequest("token cannot be empty".into()));
    }
    state.crater.set_kv("sc_access_token", &token).await?;
    Ok(Json(serde_json::json!({"status": "saved"})))
}

/// Hit SC's /me endpoint with the stored token to verify it works.
pub async fn test_sc_token(
    State(state): State<SharedState>,
) -> ApiResult<Json<serde_json::Value>> {
    let token = state.crater.get_kv("sc_access_token").await?
        .ok_or_else(|| AppError::BadRequest("no token stored".into()))?;

    let http = reqwest::Client::new();
    let resp = http
        .get("https://api.soundcloud.com/me")
        .header("Authorization", format!("OAuth {token}"))
        .send()
        .await
        .map_err(|e| AppError::Other(anyhow::anyhow!("{e}")))?;

    match resp.status().as_u16() {
        200 => {
            let body: serde_json::Value = resp.json().await
                .map_err(|e| AppError::Other(anyhow::anyhow!("{e}")))?;
            let username = body.get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            Ok(Json(serde_json::json!({
                "status":   "ok",
                "username": username,
            })))
        }
        401 | 403 => Err(AppError::BadRequest("token rejected by SoundCloud — expired or invalid".into())),
        other => Err(AppError::Other(anyhow::anyhow!("unexpected SC response: {other}"))),
    }
}
