use axum::http::{HeaderMap, StatusCode};
use std::env;
use std::fs;
use std::path::PathBuf;

pub const EXTENSION_ID: &str = "hermes-agent-teams";
pub const SIDECAR_TOKEN_HEADER: &str = "x-hermes-sidecar-token";

/// Sidecar authentication mode matching Hermes `WebUI`'s extension contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// Require valid `X-Hermes-Sidecar-Token` matching the minted `token-v1` file.
    TokenV1,
    /// Loopback-only legacy mode (for reverse-proxy auth deployments like Caddy `forward_auth`).
    Legacy,
}

impl AuthMode {
    pub fn from_env() -> Self {
        match env::var("HERMES_TEAMS_AUTH_MODE")
            .unwrap_or_else(|_| "legacy".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "token-v1" | "token_v1" | "strict" => Self::TokenV1,
            _ => Self::Legacy,
        }
    }
}

/// Resolve the `token-v1` file path in the exact order documented in Hermes `WebUI` `EXTENSIONS.md`.
pub fn resolve_token_file() -> Option<PathBuf> {
    if let Ok(explicit) = env::var("HERMES_EXT_SIDECAR_TOKEN_FILE") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    if let Ok(state_dir) = env::var("HERMES_WEBUI_STATE_DIR") {
        let trimmed = state_dir.trim();
        if !trimmed.is_empty() {
            return Some(
                PathBuf::from(trimmed)
                    .join("sidecar-auth")
                    .join(format!("{EXTENSION_ID}.token")),
            );
        }
    }
    if let Ok(hermes_home) = env::var("HERMES_HOME") {
        let trimmed = hermes_home.trim();
        if !trimmed.is_empty() {
            return Some(
                PathBuf::from(trimmed)
                    .join("webui")
                    .join("sidecar-auth")
                    .join(format!("{EXTENSION_ID}.token")),
            );
        }
    }
    if let Ok(home) = env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Some(
                PathBuf::from(trimmed)
                    .join(".hermes")
                    .join("webui")
                    .join("sidecar-auth")
                    .join(format!("{EXTENSION_ID}.token")),
            );
        }
    }
    None
}

/// Validate inbound request headers according to `AuthMode`.
/// Returns `Ok(())` on authorized request, or an HTTP status (`401` / `503`) per spec.
pub fn verify_request_auth(mode: AuthMode, headers: &HeaderMap) -> Result<(), StatusCode> {
    if mode == AuthMode::Legacy {
        return Ok(());
    }
    let Some(token_path) = resolve_token_file() else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };
    let expected = fs::read_to_string(&token_path)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .trim()
        .to_string();
    if expected.is_empty() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let provided = headers
        .get(SIDECAR_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .map_or("", str::trim);

    if constant_time_eq(expected.as_bytes(), provided.as_bytes()) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
