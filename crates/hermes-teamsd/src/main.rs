mod auth;
mod routes;
mod state;

use crate::auth::AuthMode;
use crate::routes::build_router;
use crate::state::AppState;
use axum::http::Method;
use hermes_teams_core::TeamsConfig;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let host = env::var("HERMES_TEAMS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = env::var("HERMES_TEAMS_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(17890);

    let default_workspace = env::var("HERMES_TEAMS_DEFAULT_WORKSPACE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(
            || env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            PathBuf::from,
        );

    let config = match env::var("HERMES_TEAMS_CONFIG") {
        Ok(path) if !path.trim().is_empty() => match TeamsConfig::load_yaml(&path) {
            Ok(cfg) => cfg,
            Err(err) => {
                eprintln!("Failed to load HERMES_TEAMS_CONFIG ({path}): {err}");
                std::process::exit(1);
            }
        },
        _ => {
            let default_cfg_path = env::var("HERMES_HOME")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(PathBuf::from)
                .or_else(|| {
                    env::var("HOME")
                        .ok()
                        .filter(|s| !s.trim().is_empty())
                        .map(|h| PathBuf::from(h).join(".hermes"))
                })
                .map(|h| h.join("agent-teams.yaml"));
            match default_cfg_path {
                Some(ref p) if p.is_file() => match TeamsConfig::load_yaml(p) {
                    Ok(cfg) => {
                        info!(path = %p.display(), "loaded owner agent-teams.yaml");
                        cfg
                    }
                    Err(err) => {
                        eprintln!("Failed to load {} ({err})", p.display());
                        std::process::exit(1);
                    }
                },
                _ => TeamsConfig::default(),
            }
        }
    };

    let auth_mode = AuthMode::from_env();
    let app_state = AppState::new(auth_mode, default_workspace.clone(), config);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let app = build_router(app_state).layer(cors);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("valid HERMES_TEAMS_HOST:HERMES_TEAMS_PORT");

    info!(
        %addr,
        ?auth_mode,
        default_workspace = %default_workspace.display(),
        "starting hermes-teamsd sidecar"
    );

    let listener = TcpListener::bind(addr)
        .await
        .expect("bind loopback listener");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serve hermes-teamsd");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    info!("hermes-teamsd shutting down gracefully");
}
