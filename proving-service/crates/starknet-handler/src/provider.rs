use std::{sync::Arc, time::Duration};

use eyre::Result;
use num_traits::ToPrimitive;
use starknet::{
    core::types::{BlockId, BlockTag, FunctionCall},
    macros::selector,
    providers::{JsonRpcClient, Provider, Url, jsonrpc::HttpTransport},
};
use starknet_crypto::Felt;
use tracing::{debug, error, info, instrument, warn};

use crate::{
    FeeData, FeeDataWithHash,
    account::{PitchLakeJobRequest, StarknetAccount},
    mock_data::{MockFeeDataGenerator, generate_mock_verification_hash},
};

// Default retry configuration
const DEFAULT_MAX_RETRIES: u32 = 5;
const DEFAULT_INITIAL_BACKOFF_MS: u64 = 100;
const DEFAULT_MAX_BACKOFF_MS: u64 = 10000; // 10 seconds

// Timestamp normalization
const HOUR_IN_SECONDS: u64 = 3600;

/// Normalizes a timestamp to the nearest hour boundary (rounds down)
/// Returns (normalized_timestamp, was_normalized)
fn normalize_timestamp(timestamp: u64) -> (u64, bool) {
    let normalized = (timestamp / HOUR_IN_SECONDS) * HOUR_IN_SECONDS;
    let was_normalized = normalized != timestamp;
    (normalized, was_normalized)
}

/// Configuration for StarkNet RPC connection
#[derive(Clone, Debug)]
pub struct StarkNetConfig {
    pub rpc_url: String,
    pub fossil_store_address: String,
    pub hash_store_address: Option<String>,
    pub account_address: Option<String>,
    pub account_private_key: Option<String>,
    pub max_retries: Option<u32>,
    pub initial_backoff_ms: Option<u64>,
    pub max_backoff_ms: Option<u64>,
    pub use_mock_data: bool,
}

impl StarkNetConfig {
    pub fn new(rpc_url: String, fossil_store_address: String) -> Self {
        Self {
            rpc_url,
            fossil_store_address,
            hash_store_address: None,
            account_address: None,
            account_private_key: None,
            max_retries: None,
            initial_backoff_ms: None,
            max_backoff_ms: None,
            use_mock_data: false,
        }
    }

    pub fn with_hash_store(mut self, hash_store_address: String) -> Self {
        self.hash_store_address = Some(hash_store_address);
        self
    }

    pub fn with_account(mut self, account_address: String, account_private_key: String) -> Self {
        self.account_address = Some(account_address);
        self.account_private_key = Some(account_private_key);
        self
    }

    pub fn with_retry_config(
        mut self,
        max_retries: u32,
        initial_backoff_ms: u64,
        max_backoff_ms: u64,
    ) -> Self {
        self.max_retries = Some(max_retries);
        self.initial_backoff_ms = Some(initial_backoff_ms);
        self.max_backoff_ms = Some(max_backoff_ms);
        self
    }

    pub fn with_mock_data(mut self, use_mock_data: bool) -> Self {
        self.use_mock_data = use_mock_data;
        self
    }
}

/// StarkNet provider for reading fees and hashes from the fossil_store contract
#[derive(Debug)]
pub struct StarknetProvider {
    provider: Arc<JsonRpcClient<HttpTransport>>,
    config: StarkNetConfig,
}

impl StarknetProvider {
    #[instrument(level = "debug", fields(rpc_url = %config.rpc_url))]
    pub fn new(config: StarkNetConfig) -> Result<Self> {
        debug!("Initializing StarknetProvider");

        let parsed_url = Url::parse(&config.rpc_url)?;
        debug!("Parsed RPC URL successfully");

        Ok(Self {
            provider: Arc::new(JsonRpcClient::new(HttpTransport::new(parsed_url))),
            config,
        })
    }

    /// Returns a reference to the provider's RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.config.rpc_url
    }

    /// Returns a reference to the fossil store contract address
    pub fn fossil_store_address(&self) -> &str {
        &self.config.fossil_store_address
    }

    /// Returns a reference to the hash store contract address
    pub fn hash_store_address(&self) -> Option<&str> {
        self.config.hash_store_address.as_deref()
    }

    /// Returns a clone of the provider's `JsonRpcClient`
    pub fn provider(&self) -> Arc<JsonRpcClient<HttpTransport>> {
        self.provider.clone()
    }

    /// Generic retry mechanism for any async operation with exponential backoff
    async fn with_retry<F, Fut, T>(&self, operation_name: &str, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let max_retries = self.config.max_retries.unwrap_or(DEFAULT_MAX_RETRIES);
        let mut backoff_ms = self
            .config
            .initial_backoff_ms
            .unwrap_or(DEFAULT_INITIAL_BACKOFF_MS);
        let max_backoff_ms = self.config.max_backoff_ms.unwrap_or(DEFAULT_MAX_BACKOFF_MS);
        let mut attempt = 0;

        loop {
            attempt += 1;

            match f().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!(
                            operation = operation_name,
                            attempt, "Operation succeeded after retry"
                        );
                    }
                    return Ok(result);
                }
                Err(err) => {
                    if attempt >= max_retries {
                        error!(
                            operation = operation_name,
                            attempt,
                            error = %err,
                            "Operation failed after maximum retries"
                        );
                        return Err(err);
                    }

                    // Calculate next backoff with exponential increase, capped at max_backoff_ms
                    backoff_ms = std::cmp::min(backoff_ms * 2, max_backoff_ms);

                    warn!(
                        operation = operation_name,
                        attempt,
                        next_attempt = attempt + 1,
                        backoff_ms,
                        error = %err,
                        "Operation failed, retrying after backoff"
                    );

                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
        }
    }

    /// Calls the get_avg_fees_in_range function on the fossil_store contract
    /// Returns: (first_timestamp: u64, last_timestamp: u64, fees: Array<felt252>)
    #[instrument(skip(self), level = "debug")]
    pub async fn get_avg_fees_in_range(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<FeeData> {
        // Normalize timestamps to hour boundaries (round down to nearest 3600)
        let (start_timestamp, start_normalized) = normalize_timestamp(start_timestamp);
        let (end_timestamp, end_normalized) = normalize_timestamp(end_timestamp);

        if start_normalized || end_normalized {
            warn!(
                "Timestamps normalized to hour boundaries: start={} (normalized={}), end={} (normalized={})",
                start_timestamp, start_normalized, end_timestamp, end_normalized
            );
        }

        debug!(
            start_timestamp,
            end_timestamp,
            use_mock_data = self.config.use_mock_data,
            "Fetching average fees in range"
        );

        // Return mock data if enabled
        if self.config.use_mock_data {
            info!("Using mock data for fee range request");
            let mock_generator = MockFeeDataGenerator::default();
            let fees = mock_generator.generate_mock_fee_data_as_felts()?;

            info!(
                first_timestamp = start_timestamp,
                last_timestamp = end_timestamp,
                num_fees = fees.len(),
                "Generated mock fee data"
            );

            return Ok(FeeData::new(start_timestamp, end_timestamp, fees));
        }

        self.with_retry("get_avg_fees_in_range", || async {
            let entry_point_selector = selector!("get_avg_fees_in_range");

            let data = self
                .provider
                .call(
                    FunctionCall {
                        contract_address: Felt::from_hex(&self.config.fossil_store_address)?,
                        entry_point_selector,
                        calldata: vec![Felt::from(start_timestamp), Felt::from(end_timestamp)],
                    },
                    BlockId::Tag(BlockTag::Latest),
                )
                .await?;

            // Parse the returned data according to the contract specification:
            // fn get_avg_fees_in_range(
            //     self: @TContractState, start_timestamp: u64, end_timestamp: u64,
            // ) -> (u64, u64, Array<felt252>);
            // Returns: (first_timestamp, last_timestamp, array_length, ...array_elements)
            //
            // Note: Cairo arrays are serialized with a length prefix when returned via RPC

            if data.len() < 3 {
                return Err(eyre::eyre!(
                    "Invalid response data: expected at least 3 elements (timestamp, timestamp, array_len), got {}",
                    data.len()
                ));
            }

            let first_timestamp = data[0]
                .to_u64()
                .ok_or_else(|| eyre::eyre!("Failed to convert first_timestamp to u64"))?;

            let last_timestamp = data[1]
                .to_u64()
                .ok_or_else(|| eyre::eyre!("Failed to convert last_timestamp to u64"))?;

            // data[2] is the array length, data[3..] are the actual array elements
            let array_len = data[2]
                .to_u64()
                .ok_or_else(|| eyre::eyre!("Failed to convert array length to u64"))?;

            // Extract the fee array elements (skip first 3: two timestamps + array length)
            let fees: Vec<Felt> = data[3..].to_vec();

            // Validate that the array length matches the actual number of elements
            if fees.len() != array_len as usize {
                warn!(
                    "Array length mismatch: declared {} but got {} elements",
                    array_len,
                    fees.len()
                );
            }

            info!(
                first_timestamp,
                last_timestamp,
                num_fees = fees.len(),
                raw_data_len = data.len(),
                "Retrieved fee data from contract"
            );

            Ok(FeeData::new(first_timestamp, last_timestamp, fees))
        })
        .await
    }

    /// Fetches raw fee data from the fossil store contract for RISC0 proof generation
    /// Returns raw felt values that can be directly used in RISC0 hashing
    #[instrument(skip(self), level = "debug")]
    pub async fn get_raw_fees_in_range(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<Felt>> {
        // Normalize timestamps to hour boundaries (round down to nearest 3600)
        let (start_timestamp, start_normalized) = normalize_timestamp(start_timestamp);
        let (end_timestamp, end_normalized) = normalize_timestamp(end_timestamp);

        if start_normalized || end_normalized {
            warn!(
                "Timestamps normalized to hour boundaries: start={} (normalized={}), end={} (normalized={})",
                start_timestamp, start_normalized, end_timestamp, end_normalized
            );
        }

        debug!(
            start_timestamp,
            end_timestamp,
            use_mock_data = self.config.use_mock_data,
            "Fetching raw fees for RISC0 proof generation"
        );

        // Return mock data if enabled
        if self.config.use_mock_data {
            info!("Using mock data for raw fees request");
            let mock_generator = MockFeeDataGenerator::default();
            let raw_fees = mock_generator.generate_mock_fee_data_as_felts()?;

            info!(
                num_fees = raw_fees.len(),
                "Generated mock raw fee data for RISC0 processing"
            );

            return Ok(raw_fees);
        }

        self.with_retry("get_raw_fees_in_range", || async {
            let entry_point_selector = selector!("get_avg_fees_in_range");

            let data = self
                .provider
                .call(
                    FunctionCall {
                        contract_address: Felt::from_hex(&self.config.fossil_store_address)?,
                        entry_point_selector,
                        calldata: vec![Felt::from(start_timestamp), Felt::from(end_timestamp)],
                    },
                    BlockId::Tag(BlockTag::Latest),
                )
                .await?;

            // Contract returns (first_timestamp: u64, last_timestamp: u64, array_len, ...array_elements)
            // We only need the array elements for raw fee processing
            if data.len() < 3 {
                return Err(eyre::eyre!(
                    "Invalid response data: expected at least 3 elements (first_ts, last_ts, array_len)"
                ));
            }

            let array_len = data[2]
                .to_u64()
                .ok_or_else(|| eyre::eyre!("Failed to convert array length to u64"))?;

            // Skip the first three elements (two timestamps + array length) and get the fee array
            let raw_fees = data[3..].to_vec();

            // Validate that the array length matches
            if raw_fees.len() != array_len as usize {
                warn!(
                    "Array length mismatch in raw fees: declared {} but got {} elements",
                    array_len,
                    raw_fees.len()
                );
            }

            info!(
                first_timestamp = data[0].to_u64().unwrap_or(0),
                last_timestamp = data[1].to_u64().unwrap_or(0),
                num_fees = raw_fees.len(),
                "Retrieved raw fee data for RISC0 processing"
            );

            Ok(raw_fees)
        })
        .await
    }

    /// Fetches verification hash from the hash store contract
    /// Returns the cryptographic hash used for data integrity verification in RISC0
    #[instrument(skip(self), level = "debug")]
    pub async fn get_verification_hash(&self, start_timestamp: u64) -> Result<[u32; 8]> {
        // Normalize timestamp to hour boundary (round down to nearest 3600)
        let (start_timestamp, was_normalized) = normalize_timestamp(start_timestamp);

        if was_normalized {
            warn!(
                "Timestamp normalized to hour boundary: {} (was normalized)",
                start_timestamp
            );
        }

        debug!(
            start_timestamp,
            use_mock_data = self.config.use_mock_data,
            "Fetching verification hash from hash store"
        );

        // Return mock data if enabled
        if self.config.use_mock_data {
            info!("Using mock data for verification hash request");
            let mock_hash = generate_mock_verification_hash(start_timestamp);

            info!(
                hash = ?mock_hash,
                "Generated mock verification hash"
            );

            return Ok(mock_hash);
        }

        let hash_store_address = self
            .hash_store_address()
            .ok_or_else(|| eyre::eyre!("Hash store address not configured"))?;

        debug!(hash_store_address, "Using real hash store address");

        self.with_retry("get_verification_hash", || async {
            let entry_point_selector = selector!("get_hash_stored_batched_avg_fees");

            let data = self
                .provider
                .call(
                    FunctionCall {
                        contract_address: Felt::from_hex(hash_store_address)?,
                        entry_point_selector,
                        calldata: vec![Felt::from(start_timestamp)],
                    },
                    BlockId::Tag(BlockTag::Latest),
                )
                .await?;

            // The hash store returns [u32; 8] as an array of felts
            if data.len() != 8 {
                return Err(eyre::eyre!(
                    "Invalid hash data: expected 8 u32 values, got {}",
                    data.len()
                ));
            }

            let mut hash = [0u32; 8];
            for (i, felt) in data.iter().enumerate() {
                hash[i] = felt
                    .to_u32()
                    .ok_or_else(|| eyre::eyre!("Failed to convert hash element {} to u32", i))?;
            }

            info!(
                hash = ?hash,
                "Retrieved verification hash from hash store"
            );

            Ok(hash)
        })
        .await
    }

    /// Combined function that fetches both raw fee data and verification hash
    /// This is the primary function for RISC0 proof generation
    #[instrument(skip(self), level = "debug")]
    pub async fn get_fees_with_verification(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<FeeDataWithHash> {
        // Normalize timestamps to hour boundaries (round down to nearest 3600)
        let (start_timestamp, start_normalized) = normalize_timestamp(start_timestamp);
        let (end_timestamp, end_normalized) = normalize_timestamp(end_timestamp);

        if start_normalized || end_normalized {
            warn!(
                "Timestamps normalized to hour boundaries: start={} (normalized={}), end={} (normalized={})",
                start_timestamp, start_normalized, end_timestamp, end_normalized
            );
        }

        debug!(
            start_timestamp,
            end_timestamp, "Fetching fees with verification for RISC0 proof generation"
        );

        // Fetch raw fee data and verification hash in parallel
        let (raw_fees_result, verification_hash_result) = tokio::join!(
            self.get_raw_fees_in_range(start_timestamp, end_timestamp),
            self.get_verification_hash(start_timestamp)
        );

        let raw_fees = raw_fees_result?;
        let verification_hash = verification_hash_result?;

        // Validate that we have exactly 5760 fee values as expected by RISC0
        if raw_fees.len() != 5760 {
            warn!(
                expected = 5760,
                actual = raw_fees.len(),
                "Fee data count mismatch - RISC0 proof generation may fail"
            );
        }

        // For backward compatibility, we'll set placeholder values for avg fees
        // These can be computed from raw_fees if needed by the calling code
        let avg_l1_gas_fee = 0u64; // Placeholder
        let avg_l2_gas_fee = 0u64; // Placeholder

        info!(
            num_fees = raw_fees.len(),
            hash = ?verification_hash,
            "Successfully retrieved fees with verification data"
        );

        Ok(FeeDataWithHash::new(
            raw_fees,
            verification_hash,
            avg_l1_gas_fee,
            avg_l2_gas_fee,
        ))
    }

    /// Creates a StarkNet account for write operations (hash creation, proof verification)
    fn create_account(&self) -> Result<StarknetAccount> {
        let account_address = self
            .config
            .account_address
            .as_ref()
            .ok_or_else(|| eyre::eyre!("Account address not configured"))?;
        let account_private_key = self
            .config
            .account_private_key
            .as_ref()
            .ok_or_else(|| eyre::eyre!("Account private key not configured"))?;

        StarknetAccount::new(self.provider.clone(), account_private_key, account_address)
    }

    /// Ensures all required hashes exist for the given timestamp range
    /// This is the main function called before proof generation
    #[instrument(skip(self), level = "debug")]
    pub async fn ensure_hashes_exist(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<()> {
        // Normalize timestamps to hour boundaries (round down to nearest 3600)
        let (start_timestamp, start_normalized) = normalize_timestamp(start_timestamp);
        let (end_timestamp, end_normalized) = normalize_timestamp(end_timestamp);

        if start_normalized || end_normalized {
            warn!(
                "Timestamps normalized to hour boundaries: start={} (normalized={}), end={} (normalized={})",
                start_timestamp, start_normalized, end_timestamp, end_normalized
            );
        }

        debug!(
            start_timestamp,
            end_timestamp, "Ensuring hashes exist for proof generation"
        );

        // 1. Check if final batched hash already exists
        if let Ok(_) = self.get_verification_hash(start_timestamp).await {
            debug!(
                "Final batched hash already exists for timestamp {}",
                start_timestamp
            );
            return Ok(());
        }

        info!("Final batched hash missing, checking individual batch hashes");

        // 2. Find missing individual batch hashes (32 batches of 180 fees each)
        let missing_batches = self.find_missing_batch_hashes(start_timestamp).await?;

        if !missing_batches.is_empty() {
            info!(
                count = missing_batches.len(),
                "Found missing batch hashes, creating them"
            );

            // 3. Create missing batch hashes
            let account = self.create_account()?;
            let hash_store_address = self
                .hash_store_address()
                .ok_or_else(|| eyre::eyre!("Hash store address not configured"))?;

            for batch_start in missing_batches {
                account
                    .create_batch_hash(hash_store_address, batch_start)
                    .await?;
                info!("Created batch hash for timestamp {}", batch_start);
            }
        }

        // 4. Create final batched hash from the 32 individual hashes
        info!("Creating final batched hash from individual batch hashes");
        let account = self.create_account()?;
        let hash_store_address = self
            .hash_store_address()
            .ok_or_else(|| eyre::eyre!("Hash store address not configured"))?;

        account
            .create_batched_hash(hash_store_address, start_timestamp)
            .await?;
        info!(
            "Successfully created final batched hash for timestamp {}",
            start_timestamp
        );

        Ok(())
    }

    /// Finds which individual batch hashes are missing for the 8-month period
    async fn find_missing_batch_hashes(&self, start_timestamp: u64) -> Result<Vec<u64>> {
        let mut missing_batches = Vec::new();

        // Check each of the 32 batches (180 hours = 7.5 days each)
        for i in 0..32u64 {
            let batch_start = start_timestamp + (i * 180 * 3600); // 180 hours per batch

            // Try to get the hash for this batch
            if let Err(_) = self.get_batch_hash(batch_start).await {
                missing_batches.push(batch_start);
            }
        }

        debug!(
            total_batches = 32,
            missing_count = missing_batches.len(),
            "Checked individual batch hashes"
        );

        Ok(missing_batches)
    }

    /// Gets hash for a single batch (180 fees)
    async fn get_batch_hash(&self, start_timestamp: u64) -> Result<[u32; 8]> {
        // Normalize timestamp to hour boundary (round down to nearest 3600)
        let (start_timestamp, was_normalized) = normalize_timestamp(start_timestamp);

        if was_normalized {
            warn!(
                "Timestamp normalized to hour boundary in get_batch_hash: {}",
                start_timestamp
            );
        }

        let hash_store_address = self
            .hash_store_address()
            .ok_or_else(|| eyre::eyre!("Hash store address not configured"))?;

        self.with_retry("get_batch_hash", || async {
            let entry_point_selector = selector!("get_hash_stored_avg_fees");

            let data = self
                .provider
                .call(
                    FunctionCall {
                        contract_address: Felt::from_hex(hash_store_address)?,
                        entry_point_selector,
                        calldata: vec![Felt::from(start_timestamp)],
                    },
                    BlockId::Tag(BlockTag::Latest),
                )
                .await?;

            if data.len() != 8 {
                return Err(eyre::eyre!(
                    "Invalid batch hash data: expected 8 u32 values, got {}",
                    data.len()
                ));
            }

            let mut hash = [0u32; 8];
            for (i, felt) in data.iter().enumerate() {
                hash[i] = felt.to_u32().ok_or_else(|| {
                    eyre::eyre!("Failed to convert batch hash element {} to u32", i)
                })?;
            }

            Ok(hash)
        })
        .await
    }

    /// Verifies a proof onchain using the configured account
    #[instrument(skip(self, proof), level = "debug")]
    pub async fn verify_proof_onchain(
        &self,
        verifier_address: &str,
        proof: Vec<Felt>,
        pitchlake_job_request: PitchLakeJobRequest,
    ) -> Result<Felt> {
        debug!(
            verifier_address,
            proof_length = proof.len(),
            vault_address = ?pitchlake_job_request.vault_address,
            timestamp = pitchlake_job_request.timestamp,
            "Verifying proof onchain"
        );

        let account = self.create_account()?;
        account
            .verify_proof(verifier_address, proof, pitchlake_job_request)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starknet_config_creation() {
        let config = StarkNetConfig::new(
            "http://localhost:5050".to_string(),
            "0x1234567890abcdef".to_string(),
        );

        assert_eq!(config.rpc_url, "http://localhost:5050");
        assert_eq!(config.fossil_store_address, "0x1234567890abcdef");
        assert!(config.max_retries.is_none());
    }

    #[test]
    fn test_starknet_config_with_retry_config() {
        let config = StarkNetConfig::new(
            "http://localhost:5050".to_string(),
            "0x1234567890abcdef".to_string(),
        )
        .with_retry_config(3, 200, 5000);

        assert_eq!(config.max_retries, Some(3));
        assert_eq!(config.initial_backoff_ms, Some(200));
        assert_eq!(config.max_backoff_ms, Some(5000));
    }

    #[test]
    fn test_provider_new() {
        let config = StarkNetConfig::new(
            "http://localhost:5050".to_string(),
            "0x1234567890abcdef".to_string(),
        );
        let provider = StarknetProvider::new(config);
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.rpc_url(), "http://localhost:5050");
        assert_eq!(provider.fossil_store_address(), "0x1234567890abcdef");
    }

    #[test]
    fn test_provider_new_invalid_url() {
        let config = StarkNetConfig::new(
            "not-a-valid-url".to_string(),
            "0x1234567890abcdef".to_string(),
        );
        let provider = StarknetProvider::new(config);
        assert!(provider.is_err());
    }

    #[test]
    fn test_normalize_timestamp_exact_hour() {
        let timestamp = 3600 * 100; // Exactly 100 hours
        let (normalized, was_normalized) = normalize_timestamp(timestamp);
        assert_eq!(normalized, timestamp);
        assert!(!was_normalized);
    }

    #[test]
    fn test_normalize_timestamp_rounds_down() {
        let timestamp = 3600 * 100 + 1800; // 100.5 hours
        let (normalized, was_normalized) = normalize_timestamp(timestamp);
        assert_eq!(normalized, 3600 * 100);
        assert!(was_normalized);
    }

    #[test]
    fn test_normalize_timestamp_near_hour() {
        let timestamp = 3600 * 100 + 3599; // Almost 101 hours
        let (normalized, was_normalized) = normalize_timestamp(timestamp);
        assert_eq!(normalized, 3600 * 100);
        assert!(was_normalized);
    }
}
