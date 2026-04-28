//! Official SoundCloud OAuth 2.1 token management.
//!
//! Two flows are supported:
//!
//! **Client credentials** — server-to-server, no user involvement. Replaces
//! the scraped `client_id` for search and stream operations. Tokens expire
//! in ~1 hour; the `Client` refreshes automatically.
//!
//! **Authorization code + PKCE** — user delegates playlist write access.
//! The server generates a PKCE challenge, redirects the user to SC's
//! consent screen, receives the callback code, and exchanges it for
//! `access_token` + `refresh_token`. The server (not this module) is
//! responsible for persisting the tokens; this module only handles the
//! HTTP exchanges.

use std::time::{Duration, Instant};

use crate::error::{Result, ScError};

const TOKEN_URL: &str = "https://secure.soundcloud.com/oauth/token";

// ── Token response ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TokenResponse {
    pub access_token:  String,
    pub refresh_token: Option<String>,
    /// Seconds until expiry. Typically 3599.
    pub expires_in:    Option<u64>,
    pub scope:         Option<String>,
}

/// A cached access token with its expiry clock.
#[derive(Debug, Clone)]
pub struct CachedToken {
    pub access_token:  String,
    pub refresh_token: Option<String>,
    /// Wall-clock instant after which the token should be refreshed.
    pub expires_at:    Instant,
}

impl CachedToken {
    pub fn from_response(resp: TokenResponse) -> Self {
        // Subtract 60s from the reported TTL so we refresh before expiry.
        let ttl = resp.expires_in.unwrap_or(3600).saturating_sub(60);
        Self {
            access_token:  resp.access_token,
            refresh_token: resp.refresh_token,
            expires_at:    Instant::now() + Duration::from_secs(ttl),
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

// ── OAuth config ──────────────────────────────────────────────────────────────

/// Official API credentials from https://soundcloud.com/you/apps.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id:     String,
    pub client_secret: String,
    /// Registered redirect URI — must match the portal exactly.
    pub redirect_uri:  String,
}

// ── Token exchanges ───────────────────────────────────────────────────────────

/// Obtain an access token via the client credentials flow.
/// Used for search and stream operations that don't require a user session.
pub async fn client_credentials(
    http:   &reqwest::Client,
    config: &OAuthConfig,
) -> Result<TokenResponse> {
    let resp = http
        .post(TOKEN_URL)
        .basic_auth(&config.client_id, Some(&config.client_secret))
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await?;

    parse_token_response(resp).await
}

/// Exchange an authorization code (from the PKCE callback) for tokens.
pub async fn exchange_code(
    http:          &reqwest::Client,
    config:        &OAuthConfig,
    code:          &str,
    code_verifier: &str,
) -> Result<TokenResponse> {
    let resp = http
        .post(TOKEN_URL)
        .form(&[
            ("grant_type",     "authorization_code"),
            ("client_id",      &config.client_id),
            ("client_secret",  &config.client_secret),
            ("redirect_uri",   &config.redirect_uri),
            ("code",           code),
            ("code_verifier",  code_verifier),
        ])
        .send()
        .await?;

    parse_token_response(resp).await
}

/// Refresh an expired access token using the stored refresh token.
pub async fn refresh_token(
    http:          &reqwest::Client,
    config:        &OAuthConfig,
    refresh_token: &str,
) -> Result<TokenResponse> {
    let resp = http
        .post(TOKEN_URL)
        .form(&[
            ("grant_type",    "refresh_token"),
            ("client_id",     &config.client_id),
            ("client_secret", &config.client_secret),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;

    parse_token_response(resp).await
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn parse_token_response(resp: reqwest::Response) -> Result<TokenResponse> {
    let status = resp.status();
    let body   = resp.text().await?;

    if !status.is_success() {
        return Err(ScError::Unexpected {
            status: status.as_u16(),
            body:   body.chars().take(400).collect(),
        });
    }

    serde_json::from_str(&body).map_err(|e| ScError::Unexpected {
        status: status.as_u16(),
        body:   format!("token parse error: {e} — body: {}", &body.chars().take(200).collect::<String>()),
    })
}
