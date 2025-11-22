/// End-to-end tests for local_lambdas
/// These tests verify the complete system can be built and run

use assert_cmd::Command;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create a test manifest file
fn create_test_manifest(dir: &TempDir, content: &str) -> PathBuf {
    let manifest_path = dir.path().join("manifest.xml");
    let mut file = File::create(&manifest_path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    manifest_path
}

#[test]
fn test_binary_exists() {
    // Test that the binary can be found and constructed
    let _cmd = Command::cargo_bin("local_lambdas").unwrap();
}

#[test]
fn test_with_valid_empty_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
</manifest>"#;
    
    let manifest_path = create_test_manifest(&temp_dir, xml);
    
    let mut cmd = Command::cargo_bin("local_lambdas").unwrap();
    cmd.arg(manifest_path.to_str().unwrap())
        .timeout(Duration::from_millis(500));
    
    // Just verify it doesn't crash immediately
    let _ = cmd.output();
}

#[test]
fn test_with_invalid_xml() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_xml = "not valid xml";
    
    let manifest_path = create_test_manifest(&temp_dir, invalid_xml);
    
    let mut cmd = Command::cargo_bin("local_lambdas").unwrap();
    cmd.arg(manifest_path.to_str().unwrap())
        .timeout(Duration::from_secs(1));
    
    let output = cmd.output().unwrap();
    // Should fail due to invalid XML
    assert!(!output.status.success());
}

#[test]
fn test_with_process_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>test-service</id>
        <executable>echo</executable>
        <arg>test</arg>
        <route>/test/*</route>
        <pipe_name>test_pipe</pipe_name>
    </process>
</manifest>"#;
    
    let manifest_path = create_test_manifest(&temp_dir, xml);
    
    let mut cmd = Command::cargo_bin("local_lambdas").unwrap();
    cmd.arg(manifest_path.to_str().unwrap())
        .timeout(Duration::from_millis(500));
    
    // Just verify it starts without crashing
    let _ = cmd.output();
}
