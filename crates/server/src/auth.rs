//! Crater authentication — two orthogonal concerns:
//!
//! 1. **SoundCloud PKCE flow** — lets crater act as the logged-in SC user
//!    for playlist writes. `/auth/soundcloud` → SC consent → callback →
//!    tokens stored in the `kv` table.
//!
//! 2. **Crater app password** — optional gate on the whole UI. If
//!    `CRATER_PASSWORD` is set, all routes redirect to `/login` until a
//!    valid session cookie is present.

use axum::{
    body::Body,
    extract::{Query, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use tower_sessions::Session;

use crate::state::SharedState;

// Session key used to mark an authenticated crater session.
const SESSION_KEY: &str = "crater_authed";
// Session key for the PKCE code verifier (lives only during the OAuth dance).
const PKCE_VERIFIER_KEY: &str = "pkce_verifier";

// ── PKCE helpers ──────────────────────────────────────────────────────────────

fn generate_pkce_pair() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let verifier  = URL_SAFE_NO_PAD.encode(bytes);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

// ── SoundCloud OAuth routes ───────────────────────────────────────────────────

/// GET /auth/soundcloud — redirect browser to SC consent page.
pub async fn sc_authorize(
    State(state): State<SharedState>,
    session: Session,
) -> Result<Redirect, StatusCode> {
    let cfg = state.config.sc_client_id.as_ref()
        .zip(state.config.sc_redirect_uri.as_ref())
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let (client_id, redirect_uri) = cfg;
    let (verifier, challenge) = generate_pkce_pair();

    // Store verifier in session so the callback can retrieve it.
    session.insert(PKCE_VERIFIER_KEY, verifier).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let sc_url = format!(
        "https://secure.soundcloud.com/authorize\
         ?client_id={client_id}\
         &redirect_uri={redirect_uri}\
         &response_type=code\
         &code_challenge={challenge}\
         &code_challenge_method=S256",
    );

    Ok(Redirect::to(&sc_url))
}

/// GET /auth/soundcloud/callback — exchange code for tokens, store in DB.
#[derive(serde::Deserialize)]
pub struct CallbackQuery {
    pub code:  String,
    pub error: Option<String>,
}

pub async fn sc_callback(
    State(state): State<SharedState>,
    session: Session,
    Query(q): Query<CallbackQuery>,
) -> Response {
    if let Some(err) = q.error {
        return (StatusCode::BAD_REQUEST, format!("SoundCloud error: {err}")).into_response();
    }

    let verifier = match session.get::<String>(PKCE_VERIFIER_KEY).await {
        Ok(Some(v)) => v,
        _ => return (StatusCode::BAD_REQUEST, "missing PKCE verifier — restart the auth flow").into_response(),
    };
    let _ = session.remove::<String>(PKCE_VERIFIER_KEY).await;

    let (client_id, client_secret, redirect_uri) = match (
        state.config.sc_client_id.as_ref(),
        state.config.sc_client_secret.as_ref(),
        state.config.sc_redirect_uri.as_ref(),
    ) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => return (StatusCode::SERVICE_UNAVAILABLE, "SC credentials not configured").into_response(),
    };

    let oauth_cfg = sc_client::OAuthConfig {
        client_id:     client_id.clone(),
        client_secret: client_secret.clone(),
        redirect_uri:  redirect_uri.clone(),
    };

    // Exchange code → tokens
    let http = reqwest::Client::new();
    let token_resp = match sc_client::oauth::exchange_code(&http, &oauth_cfg, &q.code, &verifier).await {
        Ok(t)  => t,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("token exchange failed: {e}")).into_response(),
    };

    // Persist tokens in the kv table
    let access  = token_resp.access_token.clone();
    let refresh = token_resp.refresh_token.clone().unwrap_or_default();
    if let Err(e) = state.crater.set_kv("sc_access_token",  &access).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    if !refresh.is_empty() {
        let _ = state.crater.set_kv("sc_refresh_token", &refresh).await;
    }

    tracing::info!("SoundCloud OAuth tokens stored");

    // Mark session as authed (covers the case where password auth is also on)
    let _ = session.insert(SESSION_KEY, true).await;

    Redirect::to("/settings").into_response()
}

// ── Crater password auth ──────────────────────────────────────────────────────

pub async fn login_page() -> axum::response::Html<&'static str> {
    axum::response::Html(LOGIN_HTML)
}

#[derive(serde::Deserialize)]
pub struct LoginForm {
    pub password: String,
}

pub async fn login_submit(
    State(state): State<SharedState>,
    session: Session,
    Form(form): Form<LoginForm>,
) -> Response {
    match &state.config.password {
        Some(expected) if *expected == form.password => {
            let _ = session.insert(SESSION_KEY, true).await;
            Redirect::to("/dig").into_response()
        }
        Some(_) => {
            // Wrong password — re-render with error
            axum::response::Html(LOGIN_HTML_ERR).into_response()
        }
        None => {
            // No password configured — auth is off, just redirect
            Redirect::to("/dig").into_response()
        }
    }
}

pub async fn logout(session: Session) -> Redirect {
    let _ = session.remove::<bool>(SESSION_KEY).await;
    Redirect::to("/login")
}

// ── Auth middleware ───────────────────────────────────────────────────────────

/// Middleware that gates all routes behind a session check when
/// `CRATER_PASSWORD` is configured.
///
/// Passes through if:
///   - no password configured (auth disabled)
///   - the request is to `/login` or `/auth/*` (don't gate the login flow)
///   - the session has `crater_authed = true`
pub async fn require_auth(
    State(state): State<SharedState>,
    session: Session,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Auth disabled — pass everything through.
    if state.config.password.is_none() {
        return next.run(request).await;
    }

    let path = request.uri().path().to_owned();

    // Never gate the login page or auth callback.
    if path == "/login" || path.starts_with("/auth/") {
        return next.run(request).await;
    }

    // Check session.
    let authed = session.get::<bool>(SESSION_KEY).await.ok().flatten().unwrap_or(false);
    if authed {
        return next.run(request).await;
    }

    Redirect::to("/login").into_response()
}

// ── Login page HTML ───────────────────────────────────────────────────────────

const LOGIN_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>crater — login</title>
<style>
  :root { --bg-0:#0f1013; --bg-1:#171920; --bg-2:#1f222b; --fg-0:#e8ebf0; --fg-1:#a0a6b0;
          --accent:#c9ad7f; --danger:#c06970; }
  * { box-sizing:border-box; margin:0; padding:0; }
  body { background:var(--bg-0); color:var(--fg-0); font:14px/1.5 system-ui,sans-serif;
         display:flex; align-items:center; justify-content:center; height:100vh; }
  .box { background:var(--bg-1); border:1px solid var(--bg-2); border-radius:6px;
         padding:32px; width:320px; }
  h1 { font-size:18px; font-weight:700; color:var(--accent); margin-bottom:24px; }
  label { display:block; font-size:11px; font-weight:600; text-transform:uppercase;
          letter-spacing:.08em; color:var(--fg-1); margin-bottom:6px; }
  input { width:100%; background:var(--bg-2); border:1px solid var(--bg-2); color:var(--fg-0);
          padding:8px 10px; border-radius:3px; font-size:14px; }
  input:focus { outline:1px solid var(--accent); border-color:var(--accent); }
  button { margin-top:16px; width:100%; background:var(--accent); color:var(--bg-0);
           border:none; border-radius:3px; padding:9px; font-size:14px; font-weight:600;
           cursor:pointer; }
  button:hover { opacity:.88; }
</style>
</head>
<body>
<div class="box">
  <h1>crater</h1>
  <form method="post" action="/login">
    <label for="pw">password</label>
    <input id="pw" name="password" type="password" autofocus autocomplete="current-password">
    <button type="submit">log in</button>
  </form>
</div>
</body>
</html>"##;

const LOGIN_HTML_ERR: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>crater — login</title>
<style>
  :root { --bg-0:#0f1013; --bg-1:#171920; --bg-2:#1f222b; --fg-0:#e8ebf0; --fg-1:#a0a6b0;
          --accent:#c9ad7f; --danger:#c06970; }
  * { box-sizing:border-box; margin:0; padding:0; }
  body { background:var(--bg-0); color:var(--fg-0); font:14px/1.5 system-ui,sans-serif;
         display:flex; align-items:center; justify-content:center; height:100vh; }
  .box { background:var(--bg-1); border:1px solid var(--bg-2); border-radius:6px;
         padding:32px; width:320px; }
  h1 { font-size:18px; font-weight:700; color:var(--accent); margin-bottom:24px; }
  .err { font-size:12px; color:var(--danger); margin-bottom:14px; }
  label { display:block; font-size:11px; font-weight:600; text-transform:uppercase;
          letter-spacing:.08em; color:var(--fg-1); margin-bottom:6px; }
  input { width:100%; background:var(--bg-2); border:1px solid var(--danger); color:var(--fg-0);
          padding:8px 10px; border-radius:3px; font-size:14px; }
  input:focus { outline:1px solid var(--accent); border-color:var(--accent); }
  button { margin-top:16px; width:100%; background:var(--accent); color:var(--bg-0);
           border:none; border-radius:3px; padding:9px; font-size:14px; font-weight:600;
           cursor:pointer; }
  button:hover { opacity:.88; }
</style>
</head>
<body>
<div class="box">
  <h1>crater</h1>
  <p class="err">incorrect password</p>
  <form method="post" action="/login">
    <label for="pw">password</label>
    <input id="pw" name="password" type="password" autofocus autocomplete="current-password">
    <button type="submit">log in</button>
  </form>
</div>
</body>
</html>"##;
