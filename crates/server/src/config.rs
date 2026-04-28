//! Server configuration via environment variables.
//!
//! All options are settable via env vars (12-factor / Docker-friendly).
//! Run `crater --help` for the full list.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "crater", about = "SoundCloud crate-digging tool")]
pub struct Config {
    #[arg(long, env = "CRATER_BIND", default_value = "0.0.0.0:8080")]
    pub bind: SocketAddr,

    /// Directory where crater.db and other data live.
    #[arg(long, env = "CRATER_DATA_DIR", default_value = "/data")]
    pub data_dir: PathBuf,

    /// Optional directory for acquired audio files (future).
    #[arg(long, env = "CRATER_MUSIC_DIR")]
    pub music_dir: Option<PathBuf>,

    // ── SoundCloud official API credentials ───────────────────────────────
    // Obtained from https://soundcloud.com/you/apps after API approval.
    // If absent, sc_client falls back to scraping a client_id from the
    // SoundCloud homepage (the old unofficial path).

    #[arg(long, env = "CRATER_SC_CLIENT_ID", hide_env_values = true)]
    pub sc_client_id: Option<String>,

    #[arg(long, env = "CRATER_SC_CLIENT_SECRET", hide_env_values = true)]
    pub sc_client_secret: Option<String>,

    /// Redirect URI registered in the SoundCloud developer portal.
    /// Must match exactly. Example: http://localhost:8080/auth/soundcloud/callback
    #[arg(long, env = "CRATER_REDIRECT_URI")]
    pub sc_redirect_uri: Option<String>,

    /// Fallback: manually captured OAuth token (see docs/04-oauth-capture.md).
    /// Used for playlist export if the PKCE flow hasn't been completed.
    #[arg(long, env = "CRATER_SC_OAUTH_TOKEN", hide_env_values = true)]
    pub sc_oauth_token: Option<String>,

    // ── Crater app password ───────────────────────────────────────────────
    // If set, all pages require a login. Suitable for exposing crater
    // beyond the LAN (behind a reverse proxy). If unset, no auth — rely
    // on network-level access control.

    /// Password to protect the crater web UI. If unset, no login required.
    #[arg(long, env = "CRATER_PASSWORD", hide_env_values = true)]
    pub password: Option<String>,

    // ── Notifications ─────────────────────────────────────────────────────

    #[arg(long, env = "CRATER_NTFY_URL")]
    pub ntfy_url: Option<String>,

    #[arg(long, env = "CRATER_NTFY_TOPIC")]
    pub ntfy_topic: Option<String>,

    /// Default IANA timezone for cron schedule evaluation.
    #[arg(long, env = "CRATER_TIMEZONE", default_value = "America/Los_Angeles")]
    pub timezone: String,

    #[arg(long, env = "CRATER_LOG", default_value = "crater=info,crater_core=info,sc_client=info")]
    pub log: String,
}
