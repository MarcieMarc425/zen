use crate::handler::node::{NodeRequest, NodeResponse, NodeResult};
use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Client;
use serde_json::Value;
use zen_expression::variable::Variable;

pub struct ApiHandler {
    trace: bool,
}

impl ApiHandler {
    pub fn new(trace: bool) -> Self {
        Self { trace }
    }

    pub async fn handle(&self, request: NodeRequest) -> NodeResult {
        let content = match &request.node.kind {
            crate::model::DecisionNodeKind::ApiNode { content } => content,
            _ => return Err(anyhow!("Invalid node type")),
        };

        let client = Client::new();
        let mut headers = HeaderMap::new();

        // Add custom headers if provided
        if let Some(custom_headers) = &content.headers {
            for (key, value) in custom_headers {
                if let (Ok(name), Ok(val)) = (
                    HeaderName::from_bytes(key.as_bytes()),
                    HeaderValue::from_str(value),
                ) {
                    headers.insert(name, val);
                }
            }
        }

        // Create request builder based on method
        let mut req_builder = match content.method.to_uppercase().as_str() {
            "GET" => client.get(&content.url),
            "POST" => client.post(&content.url),
            "PUT" => client.put(&content.url),
            "DELETE" => client.delete(&content.url),
            "PATCH" => client.patch(&content.url),
            _ => return Err(anyhow!("Unsupported HTTP method: {}", content.method)),
        };

        // Add headers to request
        req_builder = req_builder.headers(headers);

        // Add body for methods that support it
        if let Some(body) = &content.body {
            req_builder = req_builder.json(body);
        }

        // Execute request
        let response = req_builder
            .send()
            .await
            .map_err(|e| anyhow!("Failed to execute request: {}", e))?;

        // Get response status and headers before consuming the response
        let status = response.status();
        let response_headers = response.headers().clone();

        // Try to parse response as JSON
        let response_body = response
            .json::<Value>()
            .await
            .map_err(|e| anyhow!("Failed to parse response as JSON: {}", e))?;

        // Create result with response data
        Ok(NodeResponse {
            output: Variable::from(response_body),
            trace_data: if self.trace {
                Some(serde_json::json!({
                    "status": status.as_u16(),
                    "headers": response_headers
                        .iter()
                        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
                        .collect::<std::collections::HashMap<String, String>>(),
                }))
            } else {
                None
            },
        })
    }
}
