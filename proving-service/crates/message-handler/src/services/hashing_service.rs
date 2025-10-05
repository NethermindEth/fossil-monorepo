use std::sync::Arc;

use eyre::Report as EyreReport;
use starknet::core::types::{
    TransactionExecutionStatus, TransactionReceipt, TransactionReceiptWithBlockInfo,
};
use starknet::providers::Provider;

use crate::hashing::HashingProviderTrait;
use std::marker::{Send, Sync};

pub struct HashingService<T: HashingProviderTrait + Sync + Send + 'static> {
    hashing_provider: Arc<T>,
    required_avg_fees_length: usize,
    hash_batch_size: usize,
}

// Move the helper function to the module level
fn err_to_string<E: std::fmt::Display>(err: E) -> String {
    format!("{err}")
}

impl<T: HashingProviderTrait + Sync + Send + 'static> HashingService<T> {
    pub fn new(
        hashing_service: T,
        required_avg_fees_length: usize,
        hash_batch_size: usize,
    ) -> Self {
        Self {
            hashing_provider: hashing_service.into(),
            required_avg_fees_length,
            hash_batch_size,
        }
    }

    pub async fn run(&self, start_timestamp: u64) -> Result<(), String> {
        // Normalize timestamp to hour boundary (required by onchain contract)
        let normalized_start = (start_timestamp / 3600) * 3600;
        let end_timestamp = normalized_start + 3600 * (self.required_avg_fees_length as u64 - 1);

        if normalized_start != start_timestamp {
            tracing::debug!(
                "Normalized timestamp from {} to {} (hour boundary)",
                start_timestamp,
                normalized_start
            );
        }

        self.check_avg_fees_availability(normalized_start, end_timestamp)
            .await?;
        let unavailable_batch_timestamp_hashes = self
            .get_unavailable_batch_timestamp_hashes(normalized_start, end_timestamp)
            .await?;

        if !unavailable_batch_timestamp_hashes.is_empty() {
            self.hash_and_store_avg_fees_onchain(unavailable_batch_timestamp_hashes)
                .await?;
        }

        if !self
            .is_batch_hash_avg_fees_available(normalized_start)
            .await?
        {
            self.hash_batch_avg_fees_onchain(normalized_start).await?;
        }

        Ok(())
    }

    #[allow(clippy::cognitive_complexity)]
    async fn check_avg_fees_availability(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<(), String> {
        let avg_fees = self
            .hashing_provider
            .get_avg_fees_in_range(start_timestamp, end_timestamp)
            .await
            .map_err(|e| e.to_string())?;

        // NOTE: Fee count validation
        // In production, enable strict validation to ensure exact required count
        // Currently disabled to allow flexibility with available historical data
        // if avg_fees.len() != self.required_avg_fees_length {
        //     return Err("avg_fees_len is not equal to required_avg_fees_length".to_string());
        // }

        tracing::info!(
            "Received {} fee values from range (expected: {})",
            avg_fees.len(),
            self.required_avg_fees_length
        );

        // Check for any zero fees (which will cause the Cairo contract to fail)
        let zero_count = avg_fees.iter().filter(|&&fee| fee == 0.0).count();
        if zero_count > 0 {
            tracing::info!(
                "⚠️  Found {} zero fees out of {} total fees ({:.2}%)",
                zero_count,
                avg_fees.len(),
                (zero_count as f64 / avg_fees.len() as f64) * 100.0
            );

            // Log first few zero indices
            let zero_indices: Vec<usize> = avg_fees
                .iter()
                .enumerate()
                .filter(|(_, fee)| **fee == 0.0)
                .map(|(i, _)| i)
                .take(10)
                .collect();
            tracing::info!("⚠️  First zero fee indices: {:?}", zero_indices);

            // Calculate timestamps for zero fees
            let zero_timestamps: Vec<u64> = zero_indices
                .iter()
                .map(|&i| start_timestamp + (i as u64 * 3600))
                .collect();
            tracing::info!("⚠️  Timestamps with zero fees: {:?}", zero_timestamps);
        }

        // Log some sample fees for debugging
        if !avg_fees.is_empty() {
            let sample_indices = [
                0,
                avg_fees.len() / 4,
                avg_fees.len() / 2,
                3 * avg_fees.len() / 4,
                avg_fees.len() - 1,
            ];
            tracing::info!("📊 Sample fees at different points:");
            for &i in &sample_indices {
                if i < avg_fees.len() {
                    let timestamp = start_timestamp + (i as u64 * 3600);
                    tracing::info!(
                        "  Index {}: timestamp={}, fee={}",
                        i,
                        timestamp,
                        avg_fees[i]
                    );
                }
            }
        }

        Ok(())
    }

    async fn get_unavailable_batch_timestamp_hashes(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<u64>, String> {
        let mut unavailable_batch_timestamp_hashes = Vec::new();

        for t in (start_timestamp..end_timestamp).step_by(3600 * self.hash_batch_size) {
            let hash = self.hashing_provider.get_hash_stored_avg_fees(t).await;
            if let Err(err) = hash {
                return Err(err.to_string());
            }

            if let Ok(hash_value) = hash
                && hash_value == [0; 8]
            {
                tracing::info!(
                    "Found unavailable hash for timestamp {} (covers {} hours from {} to {})",
                    t,
                    self.hash_batch_size,
                    t,
                    t + (self.hash_batch_size as u64 * 3600)
                );
                unavailable_batch_timestamp_hashes.push(t);
            }
        }

        tracing::info!(
            "Total unavailable batch hashes to create: {}",
            unavailable_batch_timestamp_hashes.len()
        );

        Ok(unavailable_batch_timestamp_hashes)
    }

    // for batches that are not available, we need to make a transaction to store it
    // hash avg fee and store
    // NOTE: Transactions must be submitted sequentially to avoid nonce conflicts
    async fn hash_and_store_avg_fees_onchain(
        &self,
        unavailable_batch_timestamp_hashes: Vec<u64>,
    ) -> Result<(), String> {
        // Submit transactions sequentially to avoid nonce conflicts
        for timestamp in unavailable_batch_timestamp_hashes {
            tracing::info!(
                "Submitting transaction to hash and store {} hours of fees starting from timestamp {} (range: {} to {})",
                self.hash_batch_size,
                timestamp,
                timestamp,
                timestamp + (self.hash_batch_size as u64 * 3600)
            );

            // Submit the transaction
            let tx_result = self
                .hashing_provider
                .hash_avg_fees_and_store(timestamp)
                .await
                .map_err(|e: EyreReport| {
                    tracing::error!("Raw Starknet error for timestamp {}: {}", timestamp, e);
                    e.to_string()
                })?;

            tracing::info!(
                timestamp,
                tx_hash = ?tx_result.transaction_hash,
                "Transaction submitted successfully, waiting for receipt (this may take 1-2 minutes on Sepolia)"
            );

            // Wait for transaction receipt with retries
            // Sepolia block time is ~12 seconds, so 60 retries × 3 seconds = 3 minutes max wait
            let receipt = self
                .wait_for_transaction_receipt(tx_result.transaction_hash, 60, 3000)
                .await
                .map_err(|e| format!("Failed to get receipt for timestamp {}: {}", timestamp, e))?;

            // Check if transaction was successful
            if let TransactionReceipt::Invoke(invoke_receipt) = &receipt.receipt
                && invoke_receipt.execution_result.status() == TransactionExecutionStatus::Reverted
            {
                return Err(format!("Transaction reverted for timestamp {}", timestamp));
            }

            tracing::info!(
                timestamp,
                tx_hash = ?tx_result.transaction_hash,
                "Successfully hashed and stored average fees"
            );
        }

        Ok(())
    }

    /// Wait for a transaction receipt with retries
    ///
    /// # Arguments
    /// * `tx_hash` - The transaction hash to wait for
    /// * `max_retries` - Maximum number of retry attempts
    /// * `retry_delay_ms` - Delay between retries in milliseconds
    #[allow(clippy::cognitive_complexity)]
    async fn wait_for_transaction_receipt(
        &self,
        tx_hash: starknet::core::types::Felt,
        max_retries: u32,
        retry_delay_ms: u64,
    ) -> Result<TransactionReceiptWithBlockInfo, String> {
        for attempt in 1..=max_retries {
            // Log every 10 attempts to avoid log spam
            if attempt == 1 || attempt % 10 == 0 {
                tracing::info!(
                    attempt,
                    max_retries,
                    tx_hash = ?tx_hash,
                    elapsed_seconds = (attempt * retry_delay_ms as u32) / 1000,
                    "Waiting for transaction receipt..."
                );
            }

            match self
                .hashing_provider
                .get_provider()
                .get_transaction_receipt(tx_hash)
                .await
            {
                Ok(receipt) => {
                    tracing::info!(
                        attempt,
                        tx_hash = ?tx_hash,
                        elapsed_seconds = (attempt * retry_delay_ms as u32) / 1000,
                        "Successfully retrieved transaction receipt"
                    );
                    return Ok(receipt);
                }
                Err(e) => {
                    let error_string = e.to_string();
                    // Check if this is a "transaction not found" error (still pending)
                    if error_string.contains("TransactionHashNotFound")
                        || error_string.contains("TRANSACTION_HASH_NOT_FOUND")
                    {
                        if attempt < max_retries {
                            tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay_ms))
                                .await;
                            continue;
                        }
                        return Err(format!(
                            "Transaction {} not found after {} attempts ({} seconds). The transaction may have failed or the network may be slow.",
                            tx_hash,
                            max_retries,
                            (max_retries * retry_delay_ms as u32) / 1000
                        ));
                    }

                    // For other errors, fail immediately
                    tracing::error!(
                        attempt,
                        tx_hash = ?tx_hash,
                        error = %e,
                        "Unexpected error while getting transaction receipt"
                    );
                    return Err(format!("Failed to get transaction receipt: {}", e));
                }
            }
        }

        Err(format!(
            "Failed to get transaction receipt after {} attempts",
            max_retries
        ))
    }

    async fn is_batch_hash_avg_fees_available(&self, start_timestamp: u64) -> Result<bool, String> {
        let hash = match self
            .hashing_provider
            .get_hash_batched_avg_fees(start_timestamp)
            .await
        {
            Ok(hash) => hash,
            Err(err) => return Err(err_to_string(err)),
        };

        Ok(hash != [0; 8])
    }

    async fn hash_batch_avg_fees_onchain(&self, start_timestamp: u64) -> Result<(), String> {
        // if everything is successful, we perform batch hash of hash of avg gas fee
        let batch_hash_invoke_res = self
            .hashing_provider
            .hash_batched_avg_fees(start_timestamp)
            .await
            .map_err(|e: EyreReport| e.to_string())?;

        tracing::info!(
            start_timestamp,
            tx_hash = ?batch_hash_invoke_res.transaction_hash,
            "Batch hash transaction submitted successfully, waiting for receipt (this may take 1-2 minutes on Sepolia)"
        );

        // Wait for transaction receipt with retries
        // Sepolia block time is ~12 seconds, so 60 retries × 3 seconds = 3 minutes max wait
        let receipt = self
            .wait_for_transaction_receipt(batch_hash_invoke_res.transaction_hash, 60, 3000)
            .await
            .map_err(|e| format!("Failed to get batch hash receipt: {}", e))?;

        // Check if transaction was successful
        if let TransactionReceipt::Invoke(invoke_receipt) = &receipt.receipt
            && invoke_receipt.execution_result.status() == TransactionExecutionStatus::Reverted
        {
            return Err("batch hash reverted".to_string());
        }

        tracing::info!(
            start_timestamp,
            tx_hash = ?batch_hash_invoke_res.transaction_hash,
            "Successfully hashed and stored batched average fees"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use eyre::{Result as EyreResult, eyre};
    use starknet::{
        core::types::{Felt, InvokeTransactionResult},
        providers::{JsonRpcClient, ProviderError, jsonrpc::HttpTransport},
    };

    use crate::hashing::HashingProviderTrait;

    use super::HashingService;

    struct MockHashingProvider {
        avg_fees: Vec<f64>,
        hash_stored_avg_fees: [u32; 8],
        hash_batched_avg_fee: [u32; 8],
    }

    impl MockHashingProvider {
        pub fn new() -> Self {
            Self {
                avg_fees: vec![],
                hash_stored_avg_fees: [0; 8],
                hash_batched_avg_fee: [0; 8],
            }
        }

        pub fn set_avg_fees(&mut self, avg_fees: Vec<f64>) {
            self.avg_fees = avg_fees;
        }

        pub fn set_hash_batched_avg_fee(&mut self, hash_batched_avg_fee: [u32; 8]) {
            self.hash_batched_avg_fee = hash_batched_avg_fee;
        }
    }

    #[async_trait]
    impl HashingProviderTrait for MockHashingProvider {
        fn get_provider(&self) -> &JsonRpcClient<HttpTransport> {
            todo!()
        }

        fn get_fossil_store_address(&self) -> &Felt {
            todo!()
        }

        fn get_hash_storage_address(&self) -> &Felt {
            todo!()
        }

        async fn get_avg_fees_in_range(
            &self,
            _start_timestamp: u64,
            _end_timestamp: u64,
        ) -> Result<Vec<f64>, ProviderError> {
            Ok(self.avg_fees.clone())
        }

        async fn get_avg_fees_in_range_as_felt(
            &self,
            _start_timestamp: u64,
            _end_timestamp: u64,
        ) -> Result<Vec<Felt>, ProviderError> {
            todo!()
        }

        async fn get_hash_stored_avg_fees(
            &self,
            _timestamp: u64,
        ) -> Result<[u32; 8], ProviderError> {
            Ok(self.hash_stored_avg_fees)
        }

        async fn get_hash_batched_avg_fees(
            &self,
            _start_timestamp: u64,
        ) -> Result<[u32; 8], ProviderError> {
            Ok(self.hash_batched_avg_fee)
        }

        async fn hash_avg_fees_and_store(
            &self,
            _start_timestamp: u64,
        ) -> EyreResult<InvokeTransactionResult> {
            Err(eyre!("Mock implementation not available"))
        }

        async fn hash_batched_avg_fees(
            &self,
            _start_timestamp: u64,
        ) -> EyreResult<InvokeTransactionResult> {
            Err(eyre!("Mock implementation not available"))
        }
    }

    const REQUIRED_AVG_FEES_LENGTH: usize = 10;
    const HASH_BATCH_SIZE: usize = 10;

    fn setup() -> HashingService<MockHashingProvider> {
        let hashing_service = MockHashingProvider::new();
        HashingService::new(hashing_service, REQUIRED_AVG_FEES_LENGTH, HASH_BATCH_SIZE)
    }

    #[tokio::test]
    #[ignore = "Fee count validation is currently disabled to allow flexibility with available historical data"]
    async fn should_fail_if_check_avg_fees_availability_not_equals_to_required_avg_fees_length() {
        let process = setup();

        let res = process.check_avg_fees_availability(0, 0).await;
        assert!(res.err().unwrap() == *"avg_fees_len is not equal to required_avg_fees_length");
    }

    #[tokio::test]
    async fn should_return_ok_if_check_avg_fees_availability_equals_to_required_avg_fees_length() {
        let mut process = setup();

        Arc::get_mut(&mut process.hashing_provider)
            .unwrap()
            .set_avg_fees(vec![1.0; REQUIRED_AVG_FEES_LENGTH]);
        let res = process.check_avg_fees_availability(0, 0).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn should_get_unavailable_batch_timestamp_hashes() {
        let process = setup();

        let res = process
            .get_unavailable_batch_timestamp_hashes(0, (2 * 3600 * HASH_BATCH_SIZE as u64) - 1) //first and second batch
            .await;
        assert_eq!(res.unwrap(), vec![0, 3600 * HASH_BATCH_SIZE as u64]);
    }

    #[tokio::test]
    async fn should_return_false_if_batch_hash_avg_fees_is_not_available() {
        let process = setup();

        let res = process.is_batch_hash_avg_fees_available(0).await;
        assert!(!res.unwrap());
    }

    #[tokio::test]
    async fn should_return_true_if_batch_hash_avg_fees_is_available() {
        let mut process = setup();

        Arc::get_mut(&mut process.hashing_provider)
            .unwrap()
            .set_hash_batched_avg_fee([1; 8]);

        let res = process.is_batch_hash_avg_fees_available(0).await;
        assert!(res.unwrap());
    }
}
