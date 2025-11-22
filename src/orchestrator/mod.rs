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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProcessConfig;

    fn create_test_config(id: &str, executable: &str, pipe_name: &str) -> ProcessConfig {
        ProcessConfig {
            id: id.to_string(),
            executable: executable.to_string(),
            args: vec![],
            route: "/test".to_string(),
            pipe_name: pipe_name.to_string(),
            working_dir: None,
        }
    }

    #[test]
    fn test_orchestrator_new() {
        let orchestrator = ProcessOrchestrator::new();
        assert!(orchestrator.processes.is_empty());
    }

    #[test]
    fn test_register_process() {
        let mut orchestrator = ProcessOrchestrator::new();
        let config = create_test_config("test", "/bin/echo", "test_pipe");
        
        orchestrator.register(config.clone());
        assert_eq!(orchestrator.processes.len(), 1);
        assert!(orchestrator.processes.contains_key("test"));
    }

    #[test]
    fn test_register_multiple_processes() {
        let mut orchestrator = ProcessOrchestrator::new();
        
        orchestrator.register(create_test_config("service1", "/bin/true", "pipe1"));
        orchestrator.register(create_test_config("service2", "/bin/true", "pipe2"));
        
        assert_eq!(orchestrator.processes.len(), 2);
        assert!(orchestrator.processes.contains_key("service1"));
        assert!(orchestrator.processes.contains_key("service2"));
    }

    #[test]
    fn test_is_running_not_started() {
        let mut orchestrator = ProcessOrchestrator::new();
        orchestrator.register(create_test_config("test", "/bin/echo", "test_pipe"));
        
        assert!(!orchestrator.is_running("test"));
    }

    #[test]
    fn test_is_running_unknown_process() {
        let orchestrator = ProcessOrchestrator::new();
        assert!(!orchestrator.is_running("unknown"));
    }

    #[tokio::test]
    async fn test_start_process_success() {
        let mut orchestrator = ProcessOrchestrator::new();
        let mut config = create_test_config("test", "sleep", "test_pipe");
        config.args = vec!["0.1".to_string()];
        
        orchestrator.register(config);
        let result = orchestrator.start_process("test").await;
        
        assert!(result.is_ok());
        assert!(orchestrator.is_running("test"));
        
        // Cleanup
        orchestrator.stop_process("test").await.ok();
    }

    #[tokio::test]
    async fn test_start_process_not_found() {
        let mut orchestrator = ProcessOrchestrator::new();
        let result = orchestrator.start_process("nonexistent").await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_process_invalid_executable() {
        let mut orchestrator = ProcessOrchestrator::new();
        let config = create_test_config("test", "/nonexistent/binary", "test_pipe");
        
        orchestrator.register(config);
        let result = orchestrator.start_process("test").await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop_process() {
        let mut orchestrator = ProcessOrchestrator::new();
        let mut config = create_test_config("test", "sleep", "test_pipe");
        config.args = vec!["10".to_string()];
        
        orchestrator.register(config);
        orchestrator.start_process("test").await.ok();
        
        let result = orchestrator.stop_process("test").await;
        assert!(result.is_ok());
        assert!(!orchestrator.is_running("test"));
    }

    #[tokio::test]
    async fn test_stop_process_not_running() {
        let mut orchestrator = ProcessOrchestrator::new();
        orchestrator.register(create_test_config("test", "/bin/echo", "test_pipe"));
        
        let result = orchestrator.stop_process("test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_process_not_found() {
        let mut orchestrator = ProcessOrchestrator::new();
        let result = orchestrator.stop_process("nonexistent").await;
        
        assert!(result.is_err());
    }

    #[test]
    fn test_get_configs() {
        let mut orchestrator = ProcessOrchestrator::new();
        
        orchestrator.register(create_test_config("service1", "/bin/true", "pipe1"));
        orchestrator.register(create_test_config("service2", "/bin/true", "pipe2"));
        
        let configs = orchestrator.get_configs();
        assert_eq!(configs.len(), 2);
        
        let ids: Vec<&str> = configs.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"service1"));
        assert!(ids.contains(&"service2"));
    }

    #[test]
    fn test_get_pipe_address_static() {
        #[cfg(unix)]
        {
            let addr = ProcessOrchestrator::get_pipe_address_static("test_pipe");
            assert_eq!(addr, "/tmp/test_pipe");
        }

        #[cfg(windows)]
        {
            let addr = ProcessOrchestrator::get_pipe_address_static("test_pipe");
            assert_eq!(addr, r"\\.\pipe\test_pipe");
        }
    }
}
