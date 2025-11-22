/// Use Cases - Application-specific business rules
/// Uses domain entities and repository interfaces

use crate::domain::{HttpRequest, HttpResponse, Process, ProcessId, ProcessRepository,  
                    ProcessOrchestrationService, PipeCommunicationService, Route};
use async_trait::async_trait;
use moka::future::Cache;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Use case for initializing the system
pub struct InitializeSystemUseCase<R: ProcessRepository> {
    repository: Arc<R>,
}

impl<R: ProcessRepository> InitializeSystemUseCase<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Load all process configurations from the repository
    pub async fn execute(&self) -> Result<Vec<Process>, UseCaseError> {
        self.repository
            .load_all()
            .await
            .map_err(|e| UseCaseError::RepositoryError(e.to_string()))
    }
}

/// Use case for starting all processes
pub struct StartAllProcessesUseCase<O: ProcessOrchestrationService> {
    orchestrator: Arc<RwLock<O>>,
}

impl<O: ProcessOrchestrationService> StartAllProcessesUseCase<O> {
    pub fn new(orchestrator: Arc<RwLock<O>>) -> Self {
        Self { orchestrator }
    }

    pub async fn execute(&self) -> Result<(), UseCaseError> {
        self.orchestrator
            .write()
            .await
            .start_all()
            .await
            .map_err(|e| UseCaseError::OrchestrationError(e.to_string()))
    }
}

/// Use case for stopping all processes
pub struct StopAllProcessesUseCase<O: ProcessOrchestrationService> {
    orchestrator: Arc<RwLock<O>>,
}

impl<O: ProcessOrchestrationService> StopAllProcessesUseCase<O> {
    pub fn new(orchestrator: Arc<RwLock<O>>) -> Self {
        Self { orchestrator }
    }

    pub async fn execute(&self) -> Result<(), UseCaseError> {
        self.orchestrator
            .write()
            .await
            .stop_all()
            .await
            .map_err(|e| UseCaseError::OrchestrationError(e.to_string()))
    }
}

/// Use case for proxying HTTP requests to processes
pub struct ProxyHttpRequestUseCase<P: PipeCommunicationService> {
    pipe_service: Arc<P>,
    processes: Arc<Vec<Process>>,
    cache: Option<Cache<String, HttpResponse>>,
}

impl<P: PipeCommunicationService> ProxyHttpRequestUseCase<P> {
    pub fn new(pipe_service: Arc<P>, processes: Arc<Vec<Process>>) -> Self {
        Self::new_with_cache(pipe_service, processes, None)
    }

    pub fn new_with_cache(
        pipe_service: Arc<P>,
        processes: Arc<Vec<Process>>,
        cache_size: Option<u64>,
    ) -> Self {
        let cache = cache_size.map(|size| {
            Cache::builder()
                .max_capacity(size)
                .build()
        });
        
        Self {
            pipe_service,
            processes,
            cache,
        }
    }

    /// Execute the use case: route request to appropriate process
    /// Cache (if enabled) applies to both HTTP and named pipe communication modes
    pub async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, UseCaseError> {
        // Check cache if enabled (applies to both HTTP and pipe modes)
        if let Some(cache) = &self.cache {
            let cache_key = self.generate_cache_key(&request);
            if let Some(cached_response) = cache.get(&cache_key).await {
                tracing::debug!("Cache hit for {} (no process communication needed)", request.path);
                return Ok(cached_response);
            }
            tracing::debug!("Cache miss for {}", request.path);
        }

        use crate::domain::entities::CommunicationMode;
        use crate::domain::utils::{get_pipe_address_from_name, get_http_address_from_name};
        
        // Find matching process
        let process = self
            .find_matching_process(&request.path)
            .ok_or_else(|| UseCaseError::NoRouteFound(request.path.clone()))?;

        // Serialize request
        let request_data = self.serialize_request(&request)?;

        // Get address based on communication mode
        let address = match process.communication_mode {
            CommunicationMode::Pipe => get_pipe_address_from_name(process.pipe_name.as_str()),
            CommunicationMode::Http => get_http_address_from_name(process.pipe_name.as_str()),
        };

        tracing::debug!("Routing request to {} via {:?}: {}", 
            process.id.as_str(), process.communication_mode, address);

        // Send request through the communication channel
        let response_data = self
            .pipe_service
            .send_request(&address, request_data)
            .await
            .map_err(|e| UseCaseError::CommunicationError(e.to_string()))?;

        // Deserialize response
        let response = self.deserialize_response(response_data)?;

        // Store in cache if enabled
        if let Some(cache) = &self.cache {
            let cache_key = self.generate_cache_key(&request);
            cache.insert(cache_key, response.clone()).await;
            tracing::debug!("Cached response for {}", request.path);
        }

        Ok(response)
    }

    fn generate_cache_key(&self, request: &HttpRequest) -> String {
        format!("{}:{}", request.method.as_str(), request.path)
    }

    fn find_matching_process(&self, path: &str) -> Option<&Process> {
        self.processes
            .iter()
            .find(|p| p.route.matches(path))
    }

    fn serialize_request(&self, request: &HttpRequest) -> Result<Vec<u8>, UseCaseError> {
        use base64::{Engine as _, engine::general_purpose};
        
        let json = serde_json::json!({
            "method": request.method.as_str(),
            "uri": request.path,
            "headers": request.headers,
            "body": general_purpose::STANDARD.encode(&request.body),
        });

        serde_json::to_vec(&json)
            .map_err(|e| UseCaseError::SerializationError(e.to_string()))
    }

    fn deserialize_response(&self, data: Vec<u8>) -> Result<HttpResponse, UseCaseError> {
        use base64::{Engine as _, engine::general_purpose};
        
        let json: serde_json::Value = serde_json::from_slice(&data)
            .map_err(|e| UseCaseError::DeserializationError(e.to_string()))?;

        let status_code = json["status"].as_u64().unwrap_or(200) as u16;
        
        let headers = json["headers"]
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        v.as_str().map(|v| (k.clone(), v.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let body = json["body"]
            .as_str()
            .and_then(|s| general_purpose::STANDARD.decode(s).ok())
            .unwrap_or_default();

        Ok(HttpResponse {
            status_code,
            headers,
            body,
        })
    }

    fn get_pipe_address(pipe_name: &str) -> String {
        crate::domain::utils::get_pipe_address_from_name(pipe_name)
    }
}

/// Use case errors
#[derive(Debug)]
pub enum UseCaseError {
    RepositoryError(String),
    OrchestrationError(String),
    CommunicationError(String),
    NoRouteFound(String),
    SerializationError(String),
    DeserializationError(String),
}

impl std::fmt::Display for UseCaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UseCaseError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            UseCaseError::OrchestrationError(msg) => write!(f, "Orchestration error: {}", msg),
            UseCaseError::CommunicationError(msg) => write!(f, "Communication error: {}", msg),
            UseCaseError::NoRouteFound(path) => write!(f, "No route found for path: {}", path),
            UseCaseError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            UseCaseError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
        }
    }
}

impl std::error::Error for UseCaseError {}
