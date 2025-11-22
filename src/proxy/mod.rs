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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProcessConfig;

    fn create_test_config(id: &str, route: &str, pipe_name: &str) -> ProcessConfig {
        ProcessConfig {
            id: id.to_string(),
            executable: "test".to_string(),
            args: vec![],
            route: route.to_string(),
            pipe_name: pipe_name.to_string(),
            working_dir: None,
        }
    }

    #[test]
    fn test_proxy_state_new() {
        let configs = vec![
            create_test_config("service1", "/api/*", "pipe1"),
            create_test_config("service2", "/auth/*", "pipe2"),
        ];

        let state = ProxyState::new(configs);
        assert_eq!(state.routes.len(), 2);
    }

    #[test]
    fn test_matches_pattern_exact() {
        assert!(ProxyState::matches_pattern("/api", "/api"));
        assert!(ProxyState::matches_pattern("/api/test", "/api/test"));
        assert!(!ProxyState::matches_pattern("/api", "/api/test"));
    }

    #[test]
    fn test_matches_pattern_wildcard() {
        // Wildcard patterns
        assert!(ProxyState::matches_pattern("/api/test", "/api/*"));
        assert!(ProxyState::matches_pattern("/api/test/foo", "/api/*"));
        assert!(ProxyState::matches_pattern("/api/", "/api/*"));
        assert!(!ProxyState::matches_pattern("/other/test", "/api/*"));
        
        // Root wildcard
        assert!(ProxyState::matches_pattern("/anything", "/*"));
        assert!(ProxyState::matches_pattern("/foo/bar", "/*"));
    }

    #[test]
    fn test_matches_pattern_prefix() {
        assert!(ProxyState::matches_pattern("/api/test", "/api/"));
        assert!(ProxyState::matches_pattern("/api/", "/api/"));
        assert!(!ProxyState::matches_pattern("/other", "/api/"));
    }

    #[test]
    fn test_find_route() {
        let configs = vec![
            create_test_config("api", "/api/*", "api_pipe"),
            create_test_config("auth", "/auth/*", "auth_pipe"),
            create_test_config("root", "/*", "root_pipe"),
        ];

        let state = ProxyState::new(configs);

        // Test exact matches
        let route = state.find_route("/api/test");
        assert!(route.is_some());
        assert_eq!(route.unwrap().process_id, "api");

        let route = state.find_route("/auth/login");
        assert!(route.is_some());
        assert_eq!(route.unwrap().process_id, "auth");

        // Test fallback to root
        let route = state.find_route("/other/path");
        assert!(route.is_some());
        assert_eq!(route.unwrap().process_id, "root");
    }

    #[test]
    fn test_find_route_no_match() {
        let configs = vec![
            create_test_config("api", "/api/*", "api_pipe"),
        ];

        let state = ProxyState::new(configs);
        let route = state.find_route("/other/path");
        assert!(route.is_none());
    }

    #[test]
    fn test_find_route_first_match() {
        // When multiple patterns match, should return the first one
        let configs = vec![
            create_test_config("specific", "/api/test", "pipe1"),
            create_test_config("wildcard", "/api/*", "pipe2"),
        ];

        let state = ProxyState::new(configs);
        let route = state.find_route("/api/test");
        assert!(route.is_some());
        assert_eq!(route.unwrap().process_id, "specific");
    }

    #[test]
    fn test_get_pipe_address() {
        #[cfg(unix)]
        {
            let addr = ProxyState::get_pipe_address("test_pipe");
            assert_eq!(addr, "/tmp/test_pipe");
        }

        #[cfg(windows)]
        {
            let addr = ProxyState::get_pipe_address("test_pipe");
            assert_eq!(addr, r"\\.\pipe\test_pipe");
        }
    }

    #[tokio::test]
    async fn test_serialize_request() {
        let method = Method::GET;
        let uri = Uri::from_static("http://example.com/test");
        let headers = HeaderMap::new();
        let body = Body::from("test body");

        let result = serialize_request(method, uri, headers, body).await;
        assert!(result.is_ok());

        let data = result.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&data).unwrap();

        assert_eq!(json["method"], "GET");
        assert_eq!(json["uri"], "http://example.com/test");
        assert!(json["body"].is_string());
    }

    #[test]
    fn test_deserialize_response_success() {
        let response_json = serde_json::json!({
            "status": 200,
            "headers": {
                "Content-Type": "application/json"
            },
            "body": general_purpose::STANDARD.encode(b"test response")
        });

        let data = serde_json::to_vec(&response_json).unwrap();
        let result = deserialize_response(data);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_deserialize_response_with_status() {
        let response_json = serde_json::json!({
            "status": 404,
            "headers": {},
            "body": ""
        });

        let data = serde_json::to_vec(&response_json).unwrap();
        let result = deserialize_response(data);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_deserialize_response_invalid_json() {
        let data = b"not json".to_vec();
        let result = deserialize_response(data);
        assert!(result.is_err());
    }
}

