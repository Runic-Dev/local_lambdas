//! Process orchestration adapter - implements ProcessOrchestrationService
//! This manages the lifecycle of child processes

use crate::domain::repositories::{ProcessOrchestrationService, OrchestrationError};
use crate::domain::entities::{Process, ProcessId};
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::{Child, Command};

/// Implementation of process orchestration using tokio processes
pub struct TokioProcessOrchestrator {
    processes: HashMap<ProcessId, ManagedProcess>,
}

struct ManagedProcess {
    config: Process,
    child: Option<Child>,
}

impl Default for TokioProcessOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl TokioProcessOrchestrator {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
        }
    }

    pub fn register(&mut self, process: Process) {
        let id = process.id.clone();
        self.processes.insert(
            id,
            ManagedProcess {
                config: process,
                child: None,
            },
        );
    }
}

#[async_trait]
impl ProcessOrchestrationService for TokioProcessOrchestrator {
    async fn start_process(&mut self, id: &ProcessId) -> Result<(), OrchestrationError> {
        use crate::domain::entities::CommunicationMode;
        use crate::domain::utils::{get_pipe_address_from_name, get_http_address_from_name};
        
        let process = self
            .processes
            .get_mut(id)
            .ok_or_else(|| OrchestrationError::ProcessNotFound(id.as_str().to_string()))?;

        if process.child.is_some() {
            return Err(OrchestrationError::AlreadyRunning(id.as_str().to_string()));
        }

        tracing::info!("Starting process '{}': {} (mode: {:?})", 
            id.as_str(), process.config.executable.as_str(), process.config.communication_mode);

        let mut command = Command::new(process.config.executable.as_str());
        command.args(&process.config.arguments);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        if let Some(working_dir) = &process.config.working_directory {
            command.current_dir(working_dir.as_str());
        }

        // Set environment variable based on communication mode
        match process.config.communication_mode {
            CommunicationMode::Pipe => {
                let pipe_address = get_pipe_address_from_name(process.config.pipe_name.as_str());
                command.env("PIPE_ADDRESS", &pipe_address);
                tracing::debug!("Using pipe address: {}", pipe_address);
            }
            CommunicationMode::Http => {
                let http_address = get_http_address_from_name(process.config.pipe_name.as_str());
                command.env("HTTP_ADDRESS", &http_address);
                tracing::debug!("Using HTTP address: {}", http_address);
            }
        }

        let child = command
            .spawn()
            .map_err(|e| OrchestrationError::SpawnFailed(e.to_string()))?;

        process.child = Some(child);
        tracing::info!("Process '{}' started successfully", id.as_str());

        Ok(())
    }

    async fn stop_process(&mut self, id: &ProcessId) -> Result<(), OrchestrationError> {
        let process = self
            .processes
            .get_mut(id)
            .ok_or_else(|| OrchestrationError::ProcessNotFound(id.as_str().to_string()))?;

        if let Some(mut child) = process.child.take() {
            tracing::info!("Stopping process '{}'", id.as_str());
            child
                .kill()
                .await
                .map_err(|e| OrchestrationError::KillFailed(e.to_string()))?;
            tracing::info!("Process '{}' stopped", id.as_str());
        } else {
            tracing::warn!("Process '{}' is not running", id.as_str());
        }

        Ok(())
    }

    fn is_running(&self, id: &ProcessId) -> bool {
        self.processes
            .get(id)
            .and_then(|p| p.child.as_ref())
            .is_some()
    }

    async fn start_all(&mut self) -> Result<(), OrchestrationError> {
        let ids: Vec<ProcessId> = self.processes.keys().cloned().collect();

        for id in ids {
            if let Err(e) = self.start_process(&id).await {
                tracing::error!("Failed to start process '{}': {}", id.as_str(), e);
            }
        }

        Ok(())
    }

    async fn stop_all(&mut self) -> Result<(), OrchestrationError> {
        let ids: Vec<ProcessId> = self.processes.keys().cloned().collect();

        for id in ids {
            if let Err(e) = self.stop_process(&id).await {
                tracing::error!("Failed to stop process '{}': {}", id.as_str(), e);
            }
        }

        Ok(())
    }
}

impl Drop for TokioProcessOrchestrator {
    fn drop(&mut self) {
        for (id, process) in self.processes.iter_mut() {
            if let Some(mut child) = process.child.take() {
                tracing::info!("Cleaning up process '{}'", id.as_str());
                let _ = child.start_kill();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Executable, Route, PipeName};

    fn create_test_process(id: &str) -> Process {
        Process {
            id: ProcessId::new(id).unwrap(),
            executable: Executable::new("sleep").unwrap(),
            arguments: vec!["0.1".to_string()],
            route: Route::new("/test").unwrap(),
            pipe_name: PipeName::new("test_pipe").unwrap(),
            working_directory: None,
            communication_mode: crate::domain::entities::CommunicationMode::Pipe,
        }
    }

    #[tokio::test]
    async fn test_register_and_start_process() {
        let mut orchestrator = TokioProcessOrchestrator::new();
        let process = create_test_process("test");
        let id = process.id.clone();

        orchestrator.register(process);
        assert!(!orchestrator.is_running(&id));

        let result = orchestrator.start_process(&id).await;
        assert!(result.is_ok());
        assert!(orchestrator.is_running(&id));

        orchestrator.stop_process(&id).await.ok();
    }
}
