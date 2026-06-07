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
