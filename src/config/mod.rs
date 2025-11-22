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

    #[test]
    fn test_parse_manifest() {
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
        assert_eq!(manifest.processes[0].id, "api-service");
        assert_eq!(manifest.processes[0].route, "/api/*");
    }
}
