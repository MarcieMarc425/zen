use super::{DecisionLoader, LoaderError, LoaderResponse};
use crate::model::DecisionContent;
use azure_storage::StorageCredentials;
use azure_storage_blobs::prelude::*;
use std::future::Future;
use std::sync::Arc;

pub struct AzureBlobLoader {
    client: Arc<BlobServiceClient>,
    container_name: String,
}

impl AzureBlobLoader {
    pub fn new(
        credentials: StorageCredentials,
        account_name: impl Into<String>,
        container_name: impl Into<String>,
    ) -> Self {
        let account_name = account_name.into();
        let client = BlobServiceClient::new(account_name, credentials);
        Self {
            client: Arc::new(client),
            container_name: container_name.into(),
        }
    }

    async fn load_from_blob(&self, key: &str) -> Result<Arc<DecisionContent>, Box<LoaderError>> {
        let container_client = self.client.container_client(&self.container_name);
        let blob_client = container_client.blob_client(key);

        match blob_client.get_content().await {
            Ok(content) => {
                let decision_content: DecisionContent =
                    serde_json::from_slice(&content).map_err(|e| {
                        Box::new(LoaderError::Internal {
                            key: key.to_string(),
                            source: anyhow::anyhow!("Failed to parse blob content: {}", e),
                        })
                    })?;
                Ok(Arc::new(decision_content))
            }
            Err(e) => {
                if e.to_string().contains("404") {
                    Err(Box::new(LoaderError::NotFound(key.to_string())))
                } else {
                    Err(Box::new(LoaderError::Internal {
                        key: key.to_string(),
                        source: anyhow::anyhow!("Failed to load blob: {}", e),
                    }))
                }
            }
        }
    }
}

impl DecisionLoader for AzureBlobLoader {
    fn load<'a>(&'a self, key: &'a str) -> impl Future<Output = LoaderResponse> + 'a {
        self.load_from_blob(key)
    }
}
