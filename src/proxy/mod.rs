use crate::config::ProcessConfig;
use crate::pipes::PipeClient;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use base64::{Engine as _, engine::general_purpose};
use serde_json;

/// HTTP proxy server state
#[derive(Clone)]
pub struct ProxyState {
    routes: Arc<Vec<RouteMapping>>,
}

/// Mapping from HTTP route pattern to process pipe
#[derive(Clone)]
struct RouteMapping {
    pattern: String,
    pipe_address: String,
    process_id: String,
}

impl ProxyState {
    /// Create new proxy state from process configurations
    pub fn new(configs: Vec<ProcessConfig>) -> Self {
        let routes = configs
            .into_iter()
            .map(|config| {
                let pipe_address = Self::get_pipe_address(&config.pipe_name);
                RouteMapping {
                    pattern: config.route.clone(),
                    pipe_address,
                    process_id: config.id.clone(),
                }
            })
            .collect();

        Self {
            routes: Arc::new(routes),
        }
    }

    /// Get the pipe address for a given pipe name
    fn get_pipe_address(pipe_name: &str) -> String {
        #[cfg(windows)]
        {
            format!(r"\\.\pipe\{}", pipe_name)
        }

        #[cfg(unix)]
        {
            format!("/tmp/{}", pipe_name)
        }
    }

    /// Find the matching route for a given path
    fn find_route(&self, path: &str) -> Option<&RouteMapping> {
        self.routes.iter().find(|route| {
            Self::matches_pattern(path, &route.pattern)
        })
    }

    /// Check if a path matches a route pattern
    fn matches_pattern(path: &str, pattern: &str) -> bool {
        // Simple pattern matching: exact match or wildcard
        if pattern == path {
            return true;
        }

        // Handle wildcard patterns (e.g., "/api/*")
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2];
            return path.starts_with(prefix);
        }

        // Handle prefix patterns (e.g., "/api/")
        if pattern.ends_with('/') {
            return path.starts_with(pattern);
        }

        false
    }
}

/// Create the HTTP proxy router
pub fn create_router(state: ProxyState) -> Router {
    Router::new()
        .route("/*path", any(proxy_handler))
        .fallback(proxy_handler)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Handle incoming HTTP requests and proxy them to the appropriate process
async fn proxy_handler(
    State(state): State<ProxyState>,
    uri: Uri,
    method: Method,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let path = uri.path();
    
    tracing::debug!("Received {} request for {}", method, path);

    // Find matching route
    let route = match state.find_route(path) {
        Some(route) => route,
        None => {
            tracing::warn!("No route found for path: {}", path);
            return (
                StatusCode::NOT_FOUND,
                format!("No route configured for path: {}", path),
            )
                .into_response();
        }
    };

    tracing::info!("Routing {} {} to process '{}'", method, path, route.process_id);

    // Convert request to bytes for pipe communication
    let request_data = match serialize_request(method, uri, headers, body).await {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to serialize request: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize request: {}", e),
            )
                .into_response();
        }
    };

    // Send request through named pipe
    let client = PipeClient::new(&route.pipe_address);
    match client.send_request(request_data).await {
        Ok(response_data) => {
            // Parse response
            match deserialize_response(response_data) {
                Ok(response) => response,
                Err(e) => {
                    tracing::error!("Failed to deserialize response: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to deserialize response: {}", e),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to communicate with process '{}': {}", route.process_id, e);
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to communicate with process '{}': {}", route.process_id, e),
            )
                .into_response()
        }
    }
}

/// Serialize an HTTP request to bytes for pipe communication
async fn serialize_request(
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> anyhow::Result<Vec<u8>> {
    use axum::body::to_bytes;
    
    let body_bytes = to_bytes(body, usize::MAX).await?;
    
    // Simple serialization format: JSON-like structure
    let request = serde_json::json!({
        "method": method.as_str(),
        "uri": uri.to_string(),
        "headers": headers.iter()
            .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("")))
            .collect::<Vec<_>>(),
        "body": general_purpose::STANDARD.encode(&body_bytes),
    });

    Ok(serde_json::to_vec(&request)?)
}

/// Deserialize a response from bytes received through the pipe
fn deserialize_response(data: Vec<u8>) -> anyhow::Result<Response> {
    // Parse JSON response
    let response: serde_json::Value = serde_json::from_slice(&data)?;
    
    let status = response["status"]
        .as_u64()
        .unwrap_or(200) as u16;
    
    let body = response["body"]
        .as_str()
        .map(|s| general_purpose::STANDARD.decode(s).unwrap_or_default())
        .unwrap_or_default();
    
    let mut response_builder = Response::builder()
        .status(StatusCode::from_u16(status).unwrap_or(StatusCode::OK));
    
    // Add headers if present
    if let Some(headers) = response["headers"].as_object() {
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                response_builder = response_builder.header(key, value_str);
            }
        }
    }
    
    Ok(response_builder.body(Body::from(body))?)
}
