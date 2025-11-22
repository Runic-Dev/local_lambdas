/// Repository interfaces (Ports) - define contracts without implementation
/// These follow the Dependency Inversion Principle

use crate::domain::entities::{HttpRequest, HttpResponse, Process, ProcessId};
use async_trait::async_trait;

/// Repository for managing process configurations
#[async_trait]
pub trait ProcessRepository: Send + Sync {
    /// Load all process configurations
    async fn load_all(&self) -> Result<Vec<Process>, RepositoryError>;
}

/// Service for orchestrating processes
#[async_trait]
pub trait ProcessOrchestrationService: Send + Sync {
    /// Start a process
    async fn start_process(&mut self, id: &ProcessId) -> Result<(), OrchestrationError>;
    
    /// Stop a process
    async fn stop_process(&mut self, id: &ProcessId) -> Result<(), OrchestrationError>;
    
    /// Check if a process is running
    fn is_running(&self, id: &ProcessId) -> bool;
    
    /// Start all registered processes
    async fn start_all(&mut self) -> Result<(), OrchestrationError>;
    
    /// Stop all running processes
    async fn stop_all(&mut self) -> Result<(), OrchestrationError>;
}

/// Service for communicating with processes via named pipes
#[async_trait]
pub trait PipeCommunicationService: Send + Sync {
    /// Send a request through a named pipe and get response
    async fn send_request(
        &self,
        pipe_name: &str,
        request: Vec<u8>,
    ) -> Result<Vec<u8>, CommunicationError>;
}

/// Repository errors
#[derive(Debug)]
pub enum RepositoryError {
    NotFound(String),
    ParseError(String),
    IoError(String),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryError::NotFound(msg) => write!(f, "Not found: {}", msg),
            RepositoryError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            RepositoryError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for RepositoryError {}

/// Orchestration errors
#[derive(Debug)]
pub enum OrchestrationError {
    ProcessNotFound(String),
    AlreadyRunning(String),
    NotRunning(String),
    SpawnFailed(String),
    KillFailed(String),
}

impl std::fmt::Display for OrchestrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrchestrationError::ProcessNotFound(msg) => write!(f, "Process not found: {}", msg),
            OrchestrationError::AlreadyRunning(msg) => write!(f, "Already running: {}", msg),
            OrchestrationError::NotRunning(msg) => write!(f, "Not running: {}", msg),
            OrchestrationError::SpawnFailed(msg) => write!(f, "Spawn failed: {}", msg),
            OrchestrationError::KillFailed(msg) => write!(f, "Kill failed: {}", msg),
        }
    }
}

impl std::error::Error for OrchestrationError {}

/// Communication errors
#[derive(Debug)]
pub enum CommunicationError {
    ConnectionFailed(String),
    SendFailed(String),
    ReceiveFailed(String),
    Timeout(String),
}

impl std::fmt::Display for CommunicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommunicationError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            CommunicationError::SendFailed(msg) => write!(f, "Send failed: {}", msg),
            CommunicationError::ReceiveFailed(msg) => write!(f, "Receive failed: {}", msg),
            CommunicationError::Timeout(msg) => write!(f, "Timeout: {}", msg),
        }
    }
}

impl std::error::Error for CommunicationError {}
