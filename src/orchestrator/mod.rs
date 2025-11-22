use crate::config::ProcessConfig;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::{Child, Command};

/// Manages multiple child processes
pub struct ProcessOrchestrator {
    processes: HashMap<String, ManagedProcess>,
}

/// Represents a managed child process
struct ManagedProcess {
    config: ProcessConfig,
    child: Option<Child>,
}

impl ProcessOrchestrator {
    /// Create a new process orchestrator
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
        }
    }

    /// Register a process configuration
    pub fn register(&mut self, config: ProcessConfig) {
        let id = config.id.clone();
        self.processes.insert(
            id,
            ManagedProcess {
                config,
                child: None,
            },
        );
    }

    /// Start a registered process
    pub async fn start_process(&mut self, id: &str) -> Result<()> {
        let process = self.processes.get_mut(id)
            .context(format!("Process '{}' not found", id))?;

        if process.child.is_some() {
            tracing::warn!("Process '{}' is already running", id);
            return Ok(());
        }

        tracing::info!("Starting process '{}': {}", id, process.config.executable);

        let pipe_address = Self::get_pipe_address_static(&process.config.pipe_name);

        let mut command = Command::new(&process.config.executable);
        command.args(&process.config.args);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        if let Some(working_dir) = &process.config.working_dir {
            command.current_dir(working_dir);
        }

        // Pass pipe address as environment variable
        command.env("PIPE_ADDRESS", &pipe_address);

        let child = command.spawn()
            .context(format!("Failed to spawn process '{}'", id))?;

        process.child = Some(child);
        tracing::info!("Process '{}' started successfully", id);

        Ok(())
    }

    /// Stop a running process
    pub async fn stop_process(&mut self, id: &str) -> Result<()> {
        let process = self.processes.get_mut(id)
            .context(format!("Process '{}' not found", id))?;

        if let Some(mut child) = process.child.take() {
            tracing::info!("Stopping process '{}'", id);
            child.kill().await.context(format!("Failed to kill process '{}'", id))?;
            tracing::info!("Process '{}' stopped", id);
        } else {
            tracing::warn!("Process '{}' is not running", id);
        }

        Ok(())
    }

    /// Start all registered processes
    pub async fn start_all(&mut self) -> Result<()> {
        let ids: Vec<String> = self.processes.keys().cloned().collect();
        
        for id in ids {
            if let Err(e) = self.start_process(&id).await {
                tracing::error!("Failed to start process '{}': {}", id, e);
            }
        }

        Ok(())
    }

    /// Stop all running processes
    pub async fn stop_all(&mut self) -> Result<()> {
        let ids: Vec<String> = self.processes.keys().cloned().collect();
        
        for id in ids {
            if let Err(e) = self.stop_process(&id).await {
                tracing::error!("Failed to stop process '{}': {}", id, e);
            }
        }

        Ok(())
    }

    /// Get the pipe address for a given pipe name
    fn get_pipe_address(&self, pipe_name: &str) -> String {
        Self::get_pipe_address_static(pipe_name)
    }

    /// Static method to get pipe address
    fn get_pipe_address_static(pipe_name: &str) -> String {
        #[cfg(windows)]
        {
            format!(r"\\.\pipe\{}", pipe_name)
        }
        
        #[cfg(unix)]
        {
            format!("/tmp/{}", pipe_name)
        }
    }

    /// Check if a process is running
    pub fn is_running(&self, id: &str) -> bool {
        self.processes.get(id)
            .and_then(|p| p.child.as_ref())
            .is_some()
    }

    /// Get all process configurations
    pub fn get_configs(&self) -> Vec<&ProcessConfig> {
        self.processes.values()
            .map(|p| &p.config)
            .collect()
    }
}

impl Drop for ProcessOrchestrator {
    fn drop(&mut self) {
        // Attempt to stop all processes when the orchestrator is dropped
        for (id, process) in self.processes.iter_mut() {
            if let Some(mut child) = process.child.take() {
                tracing::info!("Cleaning up process '{}'", id);
                let _ = child.start_kill();
            }
        }
    }
}
