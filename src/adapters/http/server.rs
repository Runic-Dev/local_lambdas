/// HTTP adapter - Axum-based HTTP server controller
/// This is an interface adapter that translates HTTP requests to use cases

use crate::domain::entities::{HttpRequest, HttpResponse, HttpMethod};
use crate::use_cases::ProxyHttpRequestUseCase;
use crate::domain::PipeCommunicationService;
use axum::{
    body::Body,
    extract::State,
    http::{Method, StatusCode, Uri, HeaderMap},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

/// HTTP server state
#[derive(Clone)]
pub struct HttpServerState<P: PipeCommunicationService + Clone> {
    use_case: Arc<ProxyHttpRequestUseCase<P>>,
}

impl<P: PipeCommunicationService + Clone + 'static> HttpServerState<P> {
    pub fn new(use_case: Arc<ProxyHttpRequestUseCase<P>>) -> Self {
        Self { use_case }
    }

    pub fn create_router(self) -> Router {
        Router::new()
            .route("/*path", any(proxy_handler::<P>))
            .fallback(proxy_handler::<P>)
            .layer(TraceLayer::new_for_http())
            .with_state(self)
    }
}

/// Handle incoming HTTP requests
async fn proxy_handler<P: PipeCommunicationService + Clone>(
    State(state): State<HttpServerState<P>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Response {
    tracing::debug!("Received {} request for {}", method, uri.path());

    // Convert Axum types to domain types
    let domain_request = match convert_to_domain_request(method, uri, headers, body).await {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("Failed to convert request: {}", e);
            return (StatusCode::BAD_REQUEST, format!("Invalid request: {}", e)).into_response();
        }
    };

    // Execute use case
    match state.use_case.execute(domain_request).await {
        Ok(domain_response) => convert_to_axum_response(domain_response),
        Err(e) => {
            tracing::error!("Use case failed: {}", e);
            let status = match e {
                crate::use_cases::UseCaseError::NoRouteFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::BAD_GATEWAY,
            };
            (status, e.to_string()).into_response()
        }
    }
}

/// Convert Axum request to domain request
async fn convert_to_domain_request(
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Result<HttpRequest, String> {
    use axum::body::to_bytes;

    let body_bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|e| format!("Failed to read body: {}", e))?
        .to_vec();

    let domain_method = match method {
        Method::GET => HttpMethod::Get,
        Method::POST => HttpMethod::Post,
        Method::PUT => HttpMethod::Put,
        Method::DELETE => HttpMethod::Delete,
        Method::PATCH => HttpMethod::Patch,
        Method::HEAD => HttpMethod::Head,
        Method::OPTIONS => HttpMethod::Options,
        _ => return Err(format!("Unsupported method: {}", method)),
    };

    let domain_headers = headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|v| (k.as_str().to_string(), v.to_string()))
        })
        .collect();

    Ok(HttpRequest {
        method: domain_method,
        path: uri.path().to_string(),
        headers: domain_headers,
        body: body_bytes,
    })
}

/// Convert domain response to Axum response
fn convert_to_axum_response(domain_response: HttpResponse) -> Response {
    let mut response_builder = Response::builder()
        .status(StatusCode::from_u16(domain_response.status_code).unwrap_or(StatusCode::OK));

    for (key, value) in domain_response.headers {
        response_builder = response_builder.header(key, value);
    }

    response_builder
        .body(Body::from(domain_response.body))
        .unwrap_or_else(|e| {
            tracing::error!("Failed to build response: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        })
}
