//! Digest CRUD and manual-run trigger.

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crater_core::DigestSpec;

use crate::{
    error::{ApiResult, AppError},
    state::{DigestEvent, SharedState},
};

#[derive(serde::Deserialize)]
pub struct RunsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}
fn default_limit() -> i64 { 20 }

// ── List ──────────────────────────────────────────────────────────────────────

pub async fn list(
    State(state): State<SharedState>,
) -> ApiResult<Json<serde_json::Value>> {
    let digests = state.crater.list_digests().await?;
    Ok(Json(serde_json::to_value(digests).unwrap()))
}

// ── Get ───────────────────────────────────────────────────────────────────────

pub async fn get(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<serde_json::Value>> {
    let d = state.crater.get_digest(id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(serde_json::to_value(d).unwrap()))
}

// ── Create ────────────────────────────────────────────────────────────────────

pub async fn create(
    State(state): State<SharedState>,
    Json(spec): Json<DigestSpec>,
) -> ApiResult<Json<serde_json::Value>> {
    let d = state.crater.create_digest(spec).await?;
    Ok(Json(serde_json::to_value(d).unwrap()))
}

// ── Update ────────────────────────────────────────────────────────────────────
//
// Accepts either `{"enabled": bool}` (toggle only) or a full DigestSpec
// (replace all fields). The UI uses the former for the enable/disable switch.

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum PatchDigest {
    EnabledOnly { enabled: bool },
    FullSpec(DigestSpec),
}

pub async fn update(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    Json(patch): Json<PatchDigest>,
) -> ApiResult<Json<serde_json::Value>> {
    let d = match patch {
        PatchDigest::EnabledOnly { enabled } => {
            state.crater.get_digest(id).await?.ok_or(AppError::NotFound)?;
            state.crater.set_digest_enabled(id, enabled).await?;
            state.crater.get_digest(id).await?.ok_or(AppError::NotFound)?
        }
        PatchDigest::FullSpec(spec) => state.crater.update_digest(id, spec).await?,
    };
    Ok(Json(serde_json::to_value(d).unwrap()))
}

// ── Delete ────────────────────────────────────────────────────────────────────

pub async fn delete_one(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> ApiResult<axum::http::StatusCode> {
    state.crater.delete_digest(id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ── Manual run ────────────────────────────────────────────────────────────────

pub async fn trigger_run(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<serde_json::Value>> {
    // Verify digest exists before spawning
    state.crater.get_digest(id).await?.ok_or(AppError::NotFound)?;

    let crater  = state.crater.clone();
    let tx      = state.digest_events.clone();

    tokio::spawn(async move {
        let _ = tx.send(DigestEvent::RunStarted { digest_id: id, run_id: 0 });

        match crater.run_digest(id).await {
            Ok(run) => {
                let _ = tx.send(DigestEvent::RunCompleted {
                    digest_id:    id,
                    run_id:       run.id,
                    playlist_url: run.playlist_url,
                    track_count:  run.track_count,
                });
            }
            Err(e) => {
                let _ = tx.send(DigestEvent::RunFailed {
                    digest_id: id,
                    run_id:    0,
                    error:     e.to_string(),
                });
            }
        }
    });

    Ok(Json(serde_json::json!({ "status": "started", "digest_id": id })))
}

// ── Run history ───────────────────────────────────────────────────────────────

pub async fn list_runs(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    Query(q): Query<RunsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    state.crater.get_digest(id).await?.ok_or(AppError::NotFound)?;
    let rows = state.crater.list_digest_runs(id, q.limit).await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}
