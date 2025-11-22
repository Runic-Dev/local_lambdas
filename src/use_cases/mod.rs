/// Use Cases - Application-specific business rules
/// Uses domain entities and repository interfaces

use crate::domain::{HttpRequest, HttpResponse, Process, ProcessId, ProcessRepository,  
                    ProcessOrchestrationService, PipeCommunicationService, Route};
use async_trait::async_trait;
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
}

impl<P: PipeCommunicationService> ProxyHttpRequestUseCase<P> {
    pub fn new(pipe_service: Arc<P>, processes: Arc<Vec<Process>>) -> Self {
        Self {
            pipe_service,
            processes,
        }
    }

    /// Execute the use case: route request to appropriate process
    pub async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, UseCaseError> {
        // Find matching process
        let process = self
            .find_matching_process(&request.path)
            .ok_or_else(|| UseCaseError::NoRouteFound(request.path.clone()))?;

        // Serialize request
        let request_data = self.serialize_request(&request)?;

        // Get pipe address
        let pipe_address = Self::get_pipe_address(process.pipe_name.as_str());

        // Send request through pipe
        let response_data = self
            .pipe_service
            .send_request(&pipe_address, request_data)
            .await
            .map_err(|e| UseCaseError::CommunicationError(e.to_string()))?;

        // Deserialize response
        self.deserialize_response(response_data)
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
        #[cfg(windows)]
        {
            format!(r"\\.\pipe\{}", pipe_name)
        }

        #[cfg(unix)]
        {
            format!("/tmp/{}", pipe_name)
        }
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
