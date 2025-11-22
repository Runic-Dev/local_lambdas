mod config;
mod pipes;
mod orchestrator;
mod proxy;

use anyhow::{Context, Result};
use config::Manifest;
use orchestrator::ProcessOrchestrator;
use proxy::ProxyState;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "local_lambdas=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Local Lambdas HTTP Proxy");

    // Load manifest
    let manifest_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "manifest.xml".to_string());
    
    let manifest_path = PathBuf::from(&manifest_path);
    
    if !manifest_path.exists() {
        tracing::error!("Manifest file not found: {}", manifest_path.display());
        tracing::info!("Usage: local_lambdas [manifest.xml]");
        return Ok(());
    }

    tracing::info!("Loading manifest from: {}", manifest_path.display());
    let manifest = Manifest::from_file(&manifest_path)
        .context("Failed to load manifest")?;

    tracing::info!("Loaded {} process configuration(s)", manifest.processes.len());

    // Create orchestrator and register processes
    let mut orchestrator = ProcessOrchestrator::new();
    for config in &manifest.processes {
        tracing::info!("Registering process '{}': {} -> {}", 
            config.id, config.route, config.executable);
        orchestrator.register(config.clone());
    }

    // Start all processes
    tracing::info!("Starting all processes...");
    orchestrator.start_all().await?;

    // Give processes time to start up
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Create HTTP proxy
    let proxy_state = ProxyState::new(manifest.processes.clone());
    let app = proxy::create_router(proxy_state);

    // Bind to address
    let addr = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    
    tracing::info!("Starting HTTP proxy server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await
        .context("Failed to bind to address")?;

    tracing::info!("Local Lambdas HTTP Proxy is ready!");
    tracing::info!("Listening on http://{}", addr);

    // Run the server
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server error")?;

    // Cleanup
    tracing::info!("Shutting down...");
    orchestrator.stop_all().await?;

    Ok(())
}

/// Wait for shutdown signal (Ctrl+C)
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            tracing::info!("Received terminate signal");
        },
    }
}
