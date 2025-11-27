//! Named pipe communication adapter
//! Implements PipeCommunicationService using platform-specific named pipes

use crate::domain::repositories::{PipeCommunicationService, CommunicationError};
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(unix)]
use tokio::net::UnixStream;

/// Implementation using platform-specific named pipes
#[derive(Clone)]
pub struct NamedPipeClient;

impl Default for NamedPipeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NamedPipeClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PipeCommunicationService for NamedPipeClient {
    async fn send_request(
        &self,
        pipe_address: &str,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, CommunicationError> {
        #[cfg(windows)]
        {
            self.send_request_windows(pipe_address, data).await
        }

        #[cfg(unix)]
        {
            self.send_request_unix(pipe_address, data).await
        }
    }
}

impl NamedPipeClient {
    #[cfg(windows)]
    async fn send_request_windows(
        &self,
        pipe_address: &str,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, CommunicationError> {
        use tokio::net::windows::named_pipe::ClientOptions;

        let mut client = ClientOptions::new()
            .open(pipe_address)
            .map_err(|e| CommunicationError::ConnectionFailed(e.to_string()))?;

        client
            .write_all(&data)
            .await
            .map_err(|e| CommunicationError::SendFailed(e.to_string()))?;

        client
            .flush()
            .await
            .map_err(|e| CommunicationError::SendFailed(e.to_string()))?;

        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .await
            .map_err(|e| CommunicationError::ReceiveFailed(e.to_string()))?;

        Ok(response)
    }

    #[cfg(unix)]
    async fn send_request_unix(
        &self,
        pipe_address: &str,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, CommunicationError> {
        let mut stream = UnixStream::connect(pipe_address)
            .await
            .map_err(|e| CommunicationError::ConnectionFailed(e.to_string()))?;

        stream
            .write_all(&data)
            .await
            .map_err(|e| CommunicationError::SendFailed(e.to_string()))?;

        stream
            .flush()
            .await
            .map_err(|e| CommunicationError::SendFailed(e.to_string()))?;

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .map_err(|e| CommunicationError::ReceiveFailed(e.to_string()))?;

        Ok(response)
    }
}
