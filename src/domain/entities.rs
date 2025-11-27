//! Domain entities - pure business logic with no external dependencies

/// Represents a configured process to be orchestrated
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub id: ProcessId,
    pub executable: Executable,
    pub arguments: Vec<String>,
    pub route: Route,
    pub pipe_name: PipeName,
    pub working_directory: Option<WorkingDirectory>,
    pub communication_mode: CommunicationMode,
}

/// Value object for process identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProcessId(String);

impl ProcessId {
    pub fn new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();
        if id.is_empty() {
            return Err(DomainError::InvalidProcessId("Process ID cannot be empty".to_string()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Value object for executable path
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Executable(String);

impl Executable {
    pub fn new(path: impl Into<String>) -> Result<Self, DomainError> {
        let path = path.into();
        if path.is_empty() {
            return Err(DomainError::InvalidExecutable("Executable path cannot be empty".to_string()));
        }
        Ok(Self(path))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Value object for HTTP route pattern
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route(String);

impl Route {
    pub fn new(pattern: impl Into<String>) -> Result<Self, DomainError> {
        let pattern = pattern.into();
        if pattern.is_empty() || !pattern.starts_with('/') {
            return Err(DomainError::InvalidRoute("Route must start with /".to_string()));
        }
        Ok(Self(pattern))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if a request path matches this route pattern
    pub fn matches(&self, path: &str) -> bool {
        // Exact match
        if self.0 == path {
            return true;
        }

        // Wildcard match (e.g., "/api/*")
        if self.0.ends_with("/*") {
            let prefix = &self.0[..self.0.len() - 2];
            return path.starts_with(prefix);
        }

        // Prefix match (e.g., "/api/")
        if self.0.ends_with('/') {
            return path.starts_with(&self.0);
        }

        false
    }
}

/// Value object for named pipe identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipeName(String);

impl PipeName {
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into();
        if name.is_empty() {
            return Err(DomainError::InvalidPipeName("Pipe name cannot be empty".to_string()));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Value object for working directory
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingDirectory(String);

impl WorkingDirectory {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Communication mode for process interaction
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CommunicationMode {
    /// Use named pipes (Unix sockets or Windows named pipes)
    #[default]
    Pipe,
    /// Use HTTP protocol
    Http,
}

/// HTTP request representation
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// HTTP method
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

/// HTTP response representation
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Domain errors
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum DomainError {
    InvalidProcessId(String),
    InvalidExecutable(String),
    InvalidRoute(String),
    InvalidPipeName(String),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::InvalidProcessId(msg) => write!(f, "Invalid process ID: {}", msg),
            DomainError::InvalidExecutable(msg) => write!(f, "Invalid executable: {}", msg),
            DomainError::InvalidRoute(msg) => write!(f, "Invalid route: {}", msg),
            DomainError::InvalidPipeName(msg) => write!(f, "Invalid pipe name: {}", msg),
        }
    }
}

impl std::error::Error for DomainError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_id_validation() {
        assert!(ProcessId::new("valid-id").is_ok());
        assert!(ProcessId::new("").is_err());
    }

    #[test]
    fn test_route_matching() {
        let route = Route::new("/api/*").unwrap();
        assert!(route.matches("/api/test"));
        assert!(route.matches("/api/foo/bar"));
        assert!(!route.matches("/other/path"));
    }

    #[test]
    fn test_executable_validation() {
        assert!(Executable::new("/bin/test").is_ok());
        assert!(Executable::new("").is_err());
    }
}
