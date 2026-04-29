//! Settings API endpoints.
//!
//! GET  /api/settings               — read stored settings (ntfy, sc token status)
//! POST /api/settings/ntfy          — save ntfy URL + topic
//! POST /api/settings/sc-token      — save a manually-entered SC OAuth token
//! GET  /api/settings/sc-token/test — verify the stored token against SC's /me
//! GET  /api/stats                  — track counts by status

use axum::{extract::State, Json};

use crate::{error::{ApiResult, AppError}, state::SharedState};

pub async fn get_settings(
    State(state): State<SharedState>,
) -> ApiResult<Json<serde_json::Value>> {
    let ntfy_url       = state.crater.get_kv("ntfy_url").await?.unwrap_or_default();
    let ntfy_topic     = state.crater.get_kv("ntfy_topic").await?.unwrap_or_default();
    let sc_token_stored = state.crater.get_kv("sc_access_token").await?.is_some();
    let pkce_configured = state.config.sc_client_id.is_some()
        && state.config.sc_client_secret.is_some()
        && state.config.sc_redirect_uri.is_some();
    Ok(Json(serde_json::json!({
        "ntfy_url":        ntfy_url,
        "ntfy_topic":      ntfy_topic,
        "sc_token_stored": sc_token_stored,
        "pkce_configured": pkce_configured,
    })))
}

#[derive(serde::Deserialize)]
pub struct NtfyRequest {
    pub ntfy_url:   String,
    pub ntfy_topic: String,
}

pub async fn save_ntfy(
    State(state): State<SharedState>,
    Json(req): Json<NtfyRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let url   = req.ntfy_url.trim().to_owned();
    let topic = req.ntfy_topic.trim().to_owned();
    if url.is_empty() {
        state.crater.delete_kv("ntfy_url").await?;
    } else {
        state.crater.set_kv("ntfy_url", &url).await?;
    }
    if topic.is_empty() {
        state.crater.delete_kv("ntfy_topic").await?;
    } else {
        state.crater.set_kv("ntfy_topic", &topic).await?;
    }
    Ok(Json(serde_json::json!({"status": "saved"})))
}

pub async fn get_stats(
    State(state): State<SharedState>,
) -> ApiResult<Json<crater_core::TrackStats>> {
    Ok(Json(state.crater.track_stats().await?))
}

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
