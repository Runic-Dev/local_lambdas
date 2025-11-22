use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::PathBuf;

#[cfg(unix)]
use tokio::net::UnixListener;

#[cfg(windows)]
use tokio::net::windows::named_pipe::{ServerOptions, NamedPipeServer};

/// Cross-platform named pipe server
pub struct PipeServer {
    pipe_name: String,
    #[cfg(unix)]
    path: PathBuf,
}

impl PipeServer {
    /// Create a new pipe server with the given name
    pub fn new(pipe_name: impl Into<String>) -> Self {
        let pipe_name = pipe_name.into();
        
        #[cfg(unix)]
        let path = PathBuf::from(format!("/tmp/{}", pipe_name));
        
        Self {
            pipe_name,
            #[cfg(unix)]
            path,
        }
    }

    /// Get the pipe path/address for clients to connect to
    pub fn get_pipe_address(&self) -> String {
        #[cfg(windows)]
        {
            format!(r"\\.\pipe\{}", self.pipe_name)
        }
        
        #[cfg(unix)]
        {
            self.path.to_string_lossy().to_string()
        }
    }

    /// Start listening for connections on the named pipe
    #[cfg(windows)]
    pub async fn listen(
        &self,
        handler: impl Fn(Vec<u8>) -> Result<Vec<u8>> + Send + 'static + Clone,
    ) -> Result<()> {
        let pipe_path = format!(r"\\.\pipe\{}", self.pipe_name);
        
        loop {
            let server = ServerOptions::new()
                .first_pipe_instance(false)
                .create(&pipe_path)
                .context("Failed to create named pipe")?;

            let handler = handler.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_windows_connection(server, handler).await {
                    tracing::error!("Error handling pipe connection: {}", e);
                }
            });
        }
    }

    #[cfg(windows)]
    async fn handle_windows_connection(
        mut server: NamedPipeServer,
        handler: impl Fn(Vec<u8>) -> Result<Vec<u8>>,
    ) -> Result<()> {
        server.connect().await.context("Failed to connect pipe")?;
        
        let mut buffer = Vec::new();
        server.read_to_end(&mut buffer).await.context("Failed to read from pipe")?;
        
        let response = handler(buffer)?;
        server.write_all(&response).await.context("Failed to write to pipe")?;
        server.flush().await.context("Failed to flush pipe")?;
        
        Ok(())
    }

    #[cfg(unix)]
    pub async fn listen(
        &self,
        handler: impl Fn(Vec<u8>) -> Result<Vec<u8>> + Send + 'static + Clone,
    ) -> Result<()> {
        // Remove existing socket file if it exists
        let _ = std::fs::remove_file(&self.path);
        
        let listener = UnixListener::bind(&self.path)
            .context("Failed to bind Unix socket")?;
        
        loop {
            let (mut stream, _) = listener.accept().await
                .context("Failed to accept connection")?;
            
            let handler = handler.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_unix_connection(&mut stream, handler).await {
                    tracing::error!("Error handling pipe connection: {}", e);
                }
            });
        }
    }

    #[cfg(unix)]
    async fn handle_unix_connection(
        stream: &mut tokio::net::UnixStream,
        handler: impl Fn(Vec<u8>) -> Result<Vec<u8>>,
    ) -> Result<()> {
        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).await
            .context("Failed to read from Unix socket")?;
        
        let response = handler(buffer)?;
        stream.write_all(&response).await
            .context("Failed to write to Unix socket")?;
        stream.flush().await
            .context("Failed to flush Unix socket")?;
        
        Ok(())
    }
}

/// Client for connecting to a named pipe
pub struct PipeClient {
    pipe_address: String,
}

impl PipeClient {
    /// Create a new pipe client
    pub fn new(pipe_address: impl Into<String>) -> Self {
        Self {
            pipe_address: pipe_address.into(),
        }
    }

    /// Send a request and receive a response through the named pipe
    #[cfg(windows)]
    pub async fn send_request(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        use tokio::net::windows::named_pipe::ClientOptions;
        
        let mut client = ClientOptions::new()
            .open(&self.pipe_address)
            .context("Failed to connect to named pipe")?;
        
        client.write_all(&data).await.context("Failed to write to pipe")?;
        client.flush().await.context("Failed to flush pipe")?;
        
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.context("Failed to read from pipe")?;
        
        Ok(response)
    }

    #[cfg(unix)]
    pub async fn send_request(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        use tokio::net::UnixStream;
        
        let mut stream = UnixStream::connect(&self.pipe_address).await
            .context("Failed to connect to Unix socket")?;
        
        stream.write_all(&data).await
            .context("Failed to write to Unix socket")?;
        stream.flush().await
            .context("Failed to flush Unix socket")?;
        
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await
            .context("Failed to read from Unix socket")?;
        
        Ok(response)
    }
}
