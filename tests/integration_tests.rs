/// Integration tests for the local_lambdas HTTP proxy
/// These tests verify the interaction between multiple components

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test manifest file
fn create_test_manifest(dir: &TempDir, content: &str) -> PathBuf {
    let manifest_path = dir.path().join("manifest.xml");
    let mut file = File::create(&manifest_path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    manifest_path
}

#[test]
fn test_manifest_loading_and_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>test-service</id>
        <executable>./bin/test</executable>
        <arg>--mode</arg>
        <arg>test</arg>
        <route>/test/*</route>
        <pipe_name>test_pipe</pipe_name>
        <working_dir>./test-dir</working_dir>
    </process>
    <process>
        <id>api-service</id>
        <executable>./bin/api</executable>
        <route>/api/*</route>
        <pipe_name>api_pipe</pipe_name>
    </process>
</manifest>"#;

    let manifest_path = create_test_manifest(&temp_dir, xml);
    
    // Use the local_lambdas config module
    use local_lambdas::config::Manifest;
    
    let manifest = Manifest::from_file(manifest_path).unwrap();
    assert_eq!(manifest.processes.len(), 2);
    
    // Verify first process
    assert_eq!(manifest.processes[0].id, "test-service");
    assert_eq!(manifest.processes[0].executable, "./bin/test");
    assert_eq!(manifest.processes[0].args, vec!["--mode", "test"]);
    assert_eq!(manifest.processes[0].route, "/test/*");
    assert_eq!(manifest.processes[0].pipe_name, "test_pipe");
    assert_eq!(manifest.processes[0].working_dir, Some("./test-dir".to_string()));
    
    // Verify second process
    assert_eq!(manifest.processes[1].id, "api-service");
    assert_eq!(manifest.processes[1].args.len(), 0);
    assert!(manifest.processes[1].working_dir.is_none());
}

#[test]
fn test_orchestrator_with_manifest() {
    use local_lambdas::config::Manifest;
    use local_lambdas::orchestrator::ProcessOrchestrator;
    
    let temp_dir = TempDir::new().unwrap();
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>echo-service</id>
        <executable>echo</executable>
        <arg>test</arg>
        <route>/echo/*</route>
        <pipe_name>echo_pipe</pipe_name>
    </process>
</manifest>"#;

    let manifest_path = create_test_manifest(&temp_dir, xml);
    let manifest = Manifest::from_file(manifest_path).unwrap();
    
    let mut orchestrator = ProcessOrchestrator::new();
    
    // Register all processes from manifest
    for config in &manifest.processes {
        orchestrator.register(config.clone());
    }
    
    // Verify registration
    let configs = orchestrator.get_configs();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].id, "echo-service");
}

#[test]
fn test_proxy_state_from_manifest() {
    use local_lambdas::config::Manifest;
    use local_lambdas::proxy::ProxyState;
    
    let temp_dir = TempDir::new().unwrap();
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>api</id>
        <executable>./api</executable>
        <route>/api/*</route>
        <pipe_name>api_pipe</pipe_name>
    </process>
    <process>
        <id>auth</id>
        <executable>./auth</executable>
        <route>/auth/*</route>
        <pipe_name>auth_pipe</pipe_name>
    </process>
</manifest>"#;

    let manifest_path = create_test_manifest(&temp_dir, xml);
    let manifest = Manifest::from_file(manifest_path).unwrap();
    
    // Create proxy state from manifest
    let proxy_state = ProxyState::new(manifest.processes.clone());
    
    // The proxy_state should be ready to handle routing
    // (We can't directly test find_route as it's private, but this tests the construction)
    drop(proxy_state); // Ensure it's usable
}

#[tokio::test]
async fn test_full_orchestration_lifecycle() {
    use local_lambdas::config::ProcessConfig;
    use local_lambdas::orchestrator::ProcessOrchestrator;
    
    let mut orchestrator = ProcessOrchestrator::new();
    
    // Create a simple process config that will succeed
    let config = ProcessConfig {
        id: "sleep-test".to_string(),
        executable: "sleep".to_string(),
        args: vec!["0.5".to_string()],
        route: "/test/*".to_string(),
        pipe_name: "test_pipe".to_string(),
        working_dir: None,
    };
    
    orchestrator.register(config);
    
    // Start the process
    let result = orchestrator.start_process("sleep-test").await;
    assert!(result.is_ok(), "Failed to start process");
    assert!(orchestrator.is_running("sleep-test"));
    
    // Stop the process
    let result = orchestrator.stop_process("sleep-test").await;
    assert!(result.is_ok(), "Failed to stop process");
    assert!(!orchestrator.is_running("sleep-test"));
}

#[tokio::test]
async fn test_multiple_process_orchestration() {
    use local_lambdas::config::ProcessConfig;
    use local_lambdas::orchestrator::ProcessOrchestrator;
    
    let mut orchestrator = ProcessOrchestrator::new();
    
    // Register multiple processes
    for i in 1..=3 {
        let config = ProcessConfig {
            id: format!("service-{}", i),
            executable: "sleep".to_string(),
            args: vec!["1".to_string()],
            route: format!("/service{}/*", i),
            pipe_name: format!("pipe_{}", i),
            working_dir: None,
        };
        orchestrator.register(config);
    }
    
    // Start all processes
    let result = orchestrator.start_all().await;
    assert!(result.is_ok());
    
    // Verify all are running
    assert!(orchestrator.is_running("service-1"));
    assert!(orchestrator.is_running("service-2"));
    assert!(orchestrator.is_running("service-3"));
    
    // Stop all processes
    let result = orchestrator.stop_all().await;
    assert!(result.is_ok());
    
    // Verify all are stopped
    assert!(!orchestrator.is_running("service-1"));
    assert!(!orchestrator.is_running("service-2"));
    assert!(!orchestrator.is_running("service-3"));
}

#[test]
fn test_invalid_manifest_handling() {
    use local_lambdas::config::Manifest;
    
    let temp_dir = TempDir::new().unwrap();
    
    // Invalid XML
    let invalid_xml = "not valid xml at all";
    let manifest_path = create_test_manifest(&temp_dir, invalid_xml);
    let result = Manifest::from_file(manifest_path);
    assert!(result.is_err());
    
    // Missing required fields
    let incomplete_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>incomplete</id>
    </process>
</manifest>"#;
    
    let manifest_path = create_test_manifest(&temp_dir, incomplete_xml);
    let result = Manifest::from_file(manifest_path);
    assert!(result.is_err());
}

#[test]
fn test_empty_manifest_handling() {
    use local_lambdas::config::Manifest;
    use local_lambdas::orchestrator::ProcessOrchestrator;
    
    let temp_dir = TempDir::new().unwrap();
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
</manifest>"#;
    
    let manifest_path = create_test_manifest(&temp_dir, xml);
    let manifest = Manifest::from_file(manifest_path).unwrap();
    assert_eq!(manifest.processes.len(), 0);
    
    // Orchestrator should handle empty manifest gracefully
    let mut orchestrator = ProcessOrchestrator::new();
    for config in &manifest.processes {
        orchestrator.register(config.clone());
    }
    
    assert_eq!(orchestrator.get_configs().len(), 0);
}
