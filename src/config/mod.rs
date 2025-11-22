use serde::Deserialize;
use std::path::PathBuf;

/// Represents the manifest.xml configuration file
#[derive(Debug, Deserialize, Clone)]
#[serde(rename = "manifest")]
pub struct Manifest {
    #[serde(rename = "process", default)]
    pub processes: Vec<ProcessConfig>,
}

/// Configuration for a single process to orchestrate
#[derive(Debug, Deserialize, Clone)]
pub struct ProcessConfig {
    /// Unique identifier for the process
    pub id: String,
    
    /// Path to the executable
    pub executable: String,
    
    /// Command line arguments
    #[serde(rename = "arg", default)]
    pub args: Vec<String>,
    
    /// HTTP route pattern to match (e.g., "/api/v1/*")
    pub route: String,
    
    /// Named pipe name for communication
    pub pipe_name: String,
    
    /// Working directory for the process
    #[serde(default)]
    pub working_dir: Option<String>,
    
    /// Communication mode: "pipe" or "http" (default: "pipe")
    #[serde(default)]
    pub communication_mode: String,
}

impl Manifest {
    /// Load manifest from XML file
    pub fn from_file(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path = path.into();
        let contents = std::fs::read_to_string(&path)?;
        let manifest: Manifest = serde_xml_rs::from_str(&contents)?;
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_single_process_manifest() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>api-service</id>
        <executable>./bin/api-service.exe</executable>
        <arg>--port</arg>
        <arg>8080</arg>
        <route>/api/*</route>
        <pipe_name>api_service_pipe</pipe_name>
        <working_dir>./services/api</working_dir>
    </process>
</manifest>"#;

        let manifest: Manifest = serde_xml_rs::from_str(xml).unwrap();
        assert_eq!(manifest.processes.len(), 1);
        
        let process = &manifest.processes[0];
        assert_eq!(process.id, "api-service");
        assert_eq!(process.executable, "./bin/api-service.exe");
        assert_eq!(process.args, vec!["--port", "8080"]);
        assert_eq!(process.route, "/api/*");
        assert_eq!(process.pipe_name, "api_service_pipe");
        assert_eq!(process.working_dir, Some("./services/api".to_string()));
    }

    #[test]
    fn test_parse_multiple_processes_manifest() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>api-service</id>
        <executable>./bin/api.exe</executable>
        <route>/api/*</route>
        <pipe_name>api_pipe</pipe_name>
    </process>
    <process>
        <id>auth-service</id>
        <executable>./bin/auth.exe</executable>
        <route>/auth/*</route>
        <pipe_name>auth_pipe</pipe_name>
    </process>
</manifest>"#;

        let manifest: Manifest = serde_xml_rs::from_str(xml).unwrap();
        assert_eq!(manifest.processes.len(), 2);
        assert_eq!(manifest.processes[0].id, "api-service");
        assert_eq!(manifest.processes[1].id, "auth-service");
    }

    #[test]
    fn test_parse_process_without_optional_fields() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>minimal-service</id>
        <executable>./service</executable>
        <route>/</route>
        <pipe_name>minimal_pipe</pipe_name>
    </process>
</manifest>"#;

        let manifest: Manifest = serde_xml_rs::from_str(xml).unwrap();
        let process = &manifest.processes[0];
        assert!(process.args.is_empty());
        assert!(process.working_dir.is_none());
    }

    #[test]
    fn test_parse_empty_manifest() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
</manifest>"#;

        let manifest: Manifest = serde_xml_rs::from_str(xml).unwrap();
        assert_eq!(manifest.processes.len(), 0);
    }

    #[test]
    fn test_from_file_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>test-service</id>
        <executable>./test</executable>
        <route>/test/*</route>
        <pipe_name>test_pipe</pipe_name>
    </process>
</manifest>"#;
        temp_file.write_all(xml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let manifest = Manifest::from_file(temp_file.path()).unwrap();
        assert_eq!(manifest.processes.len(), 1);
        assert_eq!(manifest.processes[0].id, "test-service");
    }

    #[test]
    fn test_from_file_not_found() {
        let result = Manifest::from_file("/nonexistent/path/manifest.xml");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_file_invalid_xml() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"not valid xml").unwrap();
        temp_file.flush().unwrap();

        let result = Manifest::from_file(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_manifest_clone() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>test</id>
        <executable>./test</executable>
        <route>/test</route>
        <pipe_name>test_pipe</pipe_name>
    </process>
</manifest>"#;

        let manifest: Manifest = serde_xml_rs::from_str(xml).unwrap();
        let cloned = manifest.clone();
        
        assert_eq!(manifest.processes.len(), cloned.processes.len());
        assert_eq!(manifest.processes[0].id, cloned.processes[0].id);
    }
}
