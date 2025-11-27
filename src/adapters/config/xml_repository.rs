//! Config adapter - implements ProcessRepository using XML files
//! This is an infrastructure adapter

use crate::domain::repositories::{ProcessRepository, RepositoryError};
use crate::domain::entities::{Process, ProcessId, Executable, Route, PipeName, WorkingDirectory, CommunicationMode};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;

/// XML-based process repository
pub struct XmlProcessRepository {
    manifest_path: PathBuf,
}

impl XmlProcessRepository {
    pub fn new(manifest_path: impl Into<PathBuf>) -> Self {
        Self {
            manifest_path: manifest_path.into(),
        }
    }
}

#[async_trait]
impl ProcessRepository for XmlProcessRepository {
    async fn load_all(&self) -> Result<Vec<Process>, RepositoryError> {
        // Read file
        let contents = tokio::fs::read_to_string(&self.manifest_path)
            .await
            .map_err(|e| RepositoryError::IoError(e.to_string()))?;

        // Parse XML
        let manifest: ManifestDto = serde_xml_rs::from_str(&contents)
            .map_err(|e| RepositoryError::ParseError(e.to_string()))?;

        // Convert DTOs to domain entities
        manifest
            .processes
            .into_iter()
            .map(|dto| dto.into_domain())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| RepositoryError::ParseError(e.to_string()))
    }
}

/// Data Transfer Object for XML deserialization
#[derive(Debug, Deserialize)]
#[serde(rename = "manifest")]
struct ManifestDto {
    #[serde(rename = "process", default)]
    processes: Vec<ProcessDto>,
}

#[derive(Debug, Deserialize)]
struct ProcessDto {
    id: String,
    executable: String,
    #[serde(rename = "arg", default)]
    args: Vec<String>,
    route: String,
    pipe_name: String,
    #[serde(default)]
    working_dir: Option<String>,
    #[serde(default)]
    communication_mode: Option<String>,
}

impl ProcessDto {
    fn into_domain(self) -> Result<Process, String> {
        let communication_mode = match self.communication_mode.as_deref() {
            Some("http") => CommunicationMode::Http,
            Some("pipe") | None => CommunicationMode::Pipe,
            Some(other) => return Err(format!("Invalid communication mode: {}. Must be 'pipe' or 'http'", other)),
        };
        
        Ok(Process {
            id: ProcessId::new(self.id).map_err(|e| e.to_string())?,
            executable: Executable::new(self.executable).map_err(|e| e.to_string())?,
            arguments: self.args,
            route: Route::new(self.route).map_err(|e| e.to_string())?,
            pipe_name: PipeName::new(self.pipe_name).map_err(|e| e.to_string())?,
            working_directory: self.working_dir.map(WorkingDirectory::new),
            communication_mode,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_load_valid_manifest() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>test-service</id>
        <executable>./test</executable>
        <arg>--mode</arg>
        <arg>test</arg>
        <route>/test/*</route>
        <pipe_name>test_pipe</pipe_name>
    </process>
</manifest>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(xml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let repo = XmlProcessRepository::new(temp_file.path());
        let processes = repo.load_all().await.unwrap();

        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].id.as_str(), "test-service");
        assert_eq!(processes[0].arguments.len(), 2);
    }

    #[tokio::test]
    async fn test_load_invalid_xml() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"invalid xml").unwrap();
        temp_file.flush().unwrap();

        let repo = XmlProcessRepository::new(temp_file.path());
        let result = repo.load_all().await;

        assert!(result.is_err());
    }
}
