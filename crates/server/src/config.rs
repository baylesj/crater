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

    /// SoundCloud OAuth token for playlist exports.
    /// Without this, read operations work but export fails.
    #[arg(long, env = "CRATER_SC_OAUTH_TOKEN", hide_env_values = true)]
    pub sc_oauth_token: Option<String>,

    /// ntfy push notification URL (e.g. http://unraid.local:8090).
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
