/// HTTP communication adapter
/// Implements PipeCommunicationService using HTTP protocol

use crate::domain::repositories::{PipeCommunicationService, CommunicationError};
use async_trait::async_trait;

/// Implementation using HTTP protocol
#[derive(Clone)]
pub struct HttpClient;

impl HttpClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PipeCommunicationService for HttpClient {
    async fn send_request(
        &self,
        address: &str,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, CommunicationError> {
        // Parse the address - should be in format "host:port" or "127.0.0.1:port"
        let url = if address.starts_with("http://") || address.starts_with("https://") {
            address.to_string()
        } else {
            format!("http://{}", address)
        };

        tracing::debug!("Sending HTTP request to: {}", url);

        // Create HTTP client
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| CommunicationError::ConnectionFailed(e.to_string()))?;

        // Send POST request with the data
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(data)
            .send()
            .await
            .map_err(|e| CommunicationError::ConnectionFailed(e.to_string()))?;

        // Check response status
        if !response.status().is_success() {
            return Err(CommunicationError::SendFailed(format!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        // Read response body
        let response_bytes = response
            .bytes()
            .await
            .map_err(|e| CommunicationError::ReceiveFailed(e.to_string()))?
            .to_vec();

        Ok(response_bytes)
    }
}
