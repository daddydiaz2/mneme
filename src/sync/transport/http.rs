use std::time::Duration;

use crate::error::{MnemeError, Result};
use crate::sync::protocol::{SyncHello, SyncRequest, SyncResponse};

/// Transporte HTTP para sincronizacion.
pub struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
}

impl HttpTransport {
    /// Crea un nuevo HttpTransport.
    pub fn new(base_url: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| MnemeError::Http(e.to_string()))?;
        Ok(Self { client, base_url })
    }

    /// Envia saludo inicial al peer.
    pub async fn hello(&self, hello: &SyncHello) -> Result<SyncHello> {
        let url = format!("{}/api/v1/sync/hello", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(hello)
            .send()
            .await
            .map_err(|e| MnemeError::Http(e.to_string()))?;
        if !res.status().is_success() {
            return Err(MnemeError::Http(format!("hello failed: {}", res.status())));
        }
        res.json()
            .await
            .map_err(|e| MnemeError::Http(e.to_string()))
    }

    /// Solicita cambios al peer (pull).
    pub async fn pull(&self, request: &SyncRequest) -> Result<SyncResponse> {
        let url = format!("{}/api/v1/sync/pull", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(request)
            .send()
            .await
            .map_err(|e| MnemeError::Http(e.to_string()))?;
        if !res.status().is_success() {
            return Err(MnemeError::Http(format!("pull failed: {}", res.status())));
        }
        res.json()
            .await
            .map_err(|e| MnemeError::Http(e.to_string()))
    }

    /// Envia cambios al peer (push).
    pub async fn push(&self, response: &SyncResponse) -> Result<()> {
        let url = format!("{}/api/v1/sync/push", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(response)
            .send()
            .await
            .map_err(|e| MnemeError::Http(e.to_string()))?;
        if !res.status().is_success() {
            return Err(MnemeError::Http(format!("push failed: {}", res.status())));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_transport_hello_unreachable() {
        // Use invalid URL that fails connection immediately (no DNS resolution needed)
        let transport = HttpTransport::new("http://255.255.255.255:1".to_string()).unwrap();
        let hello = SyncHello {
            project: "test".to_string(),
            peer_id: uuid::Uuid::nil(),
            peer_name: "test".to_string(),
            mneme_version: "0.1.0".to_string(),
            memory_count: 0,
            heads: std::collections::HashMap::new(),
        };
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            transport.hello(&hello),
        )
        .await;
        assert!(result.is_ok(), "hello should complete within timeout");
        assert!(result.unwrap().is_err(), "hello to unreachable should error");
    }

    #[tokio::test]
    async fn test_http_transport_pull_unreachable() {
        let transport = HttpTransport::new("http://255.255.255.255:1".to_string()).unwrap();
        let request = SyncRequest {
            project: "test".to_string(),
            have: std::collections::HashMap::new(),
        };
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            transport.pull(&request),
        )
        .await;
        assert!(result.is_ok(), "pull should complete within timeout");
        assert!(result.unwrap().is_err(), "pull from unreachable should error");
    }

    #[tokio::test]
    async fn test_http_transport_push_unreachable() {
        let transport = HttpTransport::new("http://255.255.255.255:1".to_string()).unwrap();
        let response = SyncResponse {
            project: "test".to_string(),
            changes: vec![],
            tombstones: vec![],
        };
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            transport.push(&response),
        )
        .await;
        assert!(result.is_ok(), "push should complete within timeout");
        assert!(result.unwrap().is_err(), "push to unreachable should error");
    }

    #[test]
    fn test_http_transport_invalid_base_url() {
        let result = HttpTransport::new(String::new());
        // Empty base URL is technically valid for the client, just won't work for requests
        assert!(result.is_ok());
    }
}
