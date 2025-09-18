use eyre::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, instrument};
use url::Url;

#[derive(Debug)]
pub struct StarknetProvider {
    client: Client,
    rpc_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StarknetEvent {
    pub data: Vec<String>,
    pub keys: Vec<String>,
    pub block_number: Option<u64>,
    pub transaction_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventsPage {
    pub events: Vec<StarknetEvent>,
    pub continuation_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventFilter {
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub address: Option<String>,
    pub keys: Option<Vec<Vec<String>>>,
}

impl StarknetProvider {
    #[instrument(level = "debug", fields(rpc_url = %rpc_url))]
    pub fn new(rpc_url: &str) -> Result<Self> {
        debug!("Initializing StarknetProvider");

        let _parsed_url = Url::parse(rpc_url)?;
        debug!("Parsed RPC URL successfully");

        Ok(Self {
            client: Client::new(),
            rpc_url: rpc_url.to_string(),
        })
    }

    /// Returns a reference to the provider's RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    pub async fn block_number(&self) -> Result<u64> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "starknet_blockNumber",
            "params": [],
            "id": 1
        });

        let response: Value = self
            .client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?
            .json()
            .await?;

        let block_number = if let Some(block_number_str) = response["result"].as_str() {
            // Handle hex string format (e.g., "0x6")
            u64::from_str_radix(block_number_str.trim_start_matches("0x"), 16)?
        } else if let Some(block_number_num) = response["result"].as_u64() {
            // Handle direct number format (e.g., 6)
            block_number_num
        } else {
            return Err(eyre::eyre!(
                "Invalid block number response: expected string or number"
            ));
        };
        Ok(block_number)
    }

    pub async fn get_events(
        &self,
        filter: EventFilter,
        _continuation_token: Option<String>,
        _chunk_size: usize,
    ) -> Result<EventsPage> {
        let from_block = filter.from_block.map(|b| format!("0x{:x}", b));
        let to_block = filter.to_block.map(|b| format!("0x{:x}", b));

        let mut filter_params = HashMap::new();
        if let Some(from) = from_block {
            filter_params.insert("from_block", json!(from));
        }
        if let Some(to) = to_block {
            filter_params.insert("to_block", json!(to));
        }
        if let Some(address) = filter.address {
            filter_params.insert("address", json!(address));
        }
        if let Some(keys) = filter.keys {
            filter_params.insert("keys", json!(keys));
        }

        let payload = json!({
            "jsonrpc": "2.0",
            "method": "starknet_getEvents",
            "params": [{
                "filter": filter_params,
                "chunk_size": _chunk_size
            }],
            "id": 1
        });

        let response: Value = self
            .client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?
            .json()
            .await?;

        if let Some(error) = response.get("error") {
            return Err(eyre::eyre!("RPC error: {}", error));
        }

        let events_result = response["result"]["events"]
            .as_array()
            .ok_or_else(|| eyre::eyre!("Invalid events response"))?;

        let mut events = Vec::new();
        for event in events_result {
            let starknet_event = StarknetEvent {
                data: event["data"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                keys: event["keys"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                block_number: event["block_number"]
                    .as_str()
                    .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()),
                transaction_hash: event["transaction_hash"]
                    .as_str()
                    .unwrap_or("0x0")
                    .to_string(),
            };
            events.push(starknet_event);
        }

        Ok(EventsPage {
            events,
            continuation_token: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_new() {
        let rpc_url = "http://localhost:5050";
        let provider = StarknetProvider::new(rpc_url);
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.rpc_url(), rpc_url);
    }

    #[test]
    fn test_provider_new_invalid_url() {
        let rpc_url = "not-a-valid-url";
        let provider = StarknetProvider::new(rpc_url);
        assert!(provider.is_err());
    }
}
