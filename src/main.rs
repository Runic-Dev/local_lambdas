/// Main entry point using Clean Architecture
/// This file is part of the outermost layer (Frameworks & Drivers)

mod domain;
mod use_cases;
mod adapters;
mod infrastructure;

// Legacy modules for backward compatibility
mod config;
mod orchestrator;
mod pipes;
mod proxy;

use adapters::{XmlProcessRepository, TokioProcessOrchestrator, HttpServerState};
use infrastructure::NamedPipeClient;
use use_cases::{InitializeSystemUseCase, StartAllProcessesUseCase, StopAllProcessesUseCase, ProxyHttpRequestUseCase};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "local_lambdas=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Local Lambdas HTTP Proxy (Clean Architecture)");

    // Parse command line arguments
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

    // ========== Dependency Injection Setup ==========
    
    // Infrastructure Layer
    let process_repository = Arc::new(XmlProcessRepository::new(&manifest_path));
    let pipe_service = Arc::new(NamedPipeClient::new());
    
    // Use Cases Layer
    let init_use_case = InitializeSystemUseCase::new(process_repository.clone());
    
    // Execute initialization use case
    let processes = init_use_case.execute().await?;
    tracing::info!("Loaded {} process configuration(s)", processes.len());

    // Create orchestrator and register processes
    let mut orchestrator = TokioProcessOrchestrator::new();
    for process in &processes {
        tracing::info!("Registering process '{}': {} -> {}", 
            process.id.as_str(), process.route.as_str(), process.executable.as_str());
        orchestrator.register(process.clone());
    }
    
    let orchestrator = Arc::new(RwLock::new(orchestrator));
    
    // Use case for starting processes
    let start_use_case = StartAllProcessesUseCase::new(orchestrator.clone());
    
    tracing::info!("Starting all processes...");
    start_use_case.execute().await?;

    // Give processes time to start up
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Create proxy use case
    let processes_arc = Arc::new(processes);
    let proxy_use_case = Arc::new(ProxyHttpRequestUseCase::new(
        pipe_service.clone(),
        processes_arc,
    ));

    // Adapters Layer - HTTP Server
    let server_state = HttpServerState::new(proxy_use_case);
    let app = server_state.create_router();

    // Bind to address
    let addr = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    
    tracing::info!("Starting HTTP proxy server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Local Lambdas HTTP Proxy is ready!");
    tracing::info!("Listening on http://{}", addr);

    // Run the server
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Cleanup
    tracing::info!("Shutting down...");
    let stop_use_case = StopAllProcessesUseCase::new(orchestrator);
    stop_use_case.execute().await?;

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
