use std::{sync::Arc, time::Duration};

use eyre::Result;
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::{codec::Encode, types::Call},
    macros::selector,
    providers::{JsonRpcClient, jsonrpc::HttpTransport},
    signers::{LocalWallet, SigningKey},
};
use starknet_crypto::Felt;
use tracing::{debug, info, instrument, warn};

/// Katana devnet chain ID
const KATANA_CHAIN_ID: Felt = Felt::from_raw([0x4b4154414e41, 0, 0, 0]);

// Timestamp normalization
const HOUR_IN_SECONDS: u64 = 3600;

/// Normalizes a timestamp to the nearest hour boundary (rounds down)
/// Returns (normalized_timestamp, was_normalized)
fn normalize_timestamp(timestamp: u64) -> (u64, bool) {
    let normalized = (timestamp / HOUR_IN_SECONDS) * HOUR_IN_SECONDS;
    let was_normalized = normalized != timestamp;
    (normalized, was_normalized)
}

/// PitchLakeJobRequest struct matching the Cairo contract definition
#[derive(Debug, Clone, Encode)]
pub struct PitchLakeJobRequest {
    pub vault_address: Felt,
    pub timestamp: u64,
    pub program_id: Felt, // 'PITCH_LAKE_V1'
}

/// Helper function to convert hex string to Felt (similar to fossil-light-client)
fn felt(str: &str) -> Result<Felt> {
    Felt::from_hex(str).map_err(|_| eyre::eyre!("Invalid hex string: {}", str))
}

pub struct StarknetAccount {
    account: SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>,
}

impl StarknetAccount {
    #[instrument(skip(provider, account_private_key), fields(address = %account_address), level = "debug")]
    pub fn new(
        provider: Arc<JsonRpcClient<HttpTransport>>,
        account_private_key: &str,
        account_address: &str,
    ) -> Result<Self> {
        debug!("Creating new Starknet account for proving service");

        let private_key = felt(account_private_key)?;
        debug!("Private key converted to felt");

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));
        let address = felt(account_address)?;

        debug!(
            chain_id = ?KATANA_CHAIN_ID,
            encoding = ?ExecutionEncoding::New,
            "Initializing SingleOwnerAccount"
        );

        let account = SingleOwnerAccount::new(
            provider,
            signer,
            address,
            KATANA_CHAIN_ID,
            ExecutionEncoding::New,
        );

        debug!("Starknet account successfully created");
        Ok(Self { account })
    }

    /// Creates hash for 180 avg fees batch
    #[instrument(skip(self), level = "debug")]
    pub async fn create_batch_hash(
        &self,
        hash_store_address: &str,
        start_timestamp: u64,
    ) -> Result<Felt> {
        // Normalize timestamp to hour boundary (round down to nearest 3600)
        let (start_timestamp, was_normalized) = normalize_timestamp(start_timestamp);

        if was_normalized {
            warn!(
                "Timestamp normalized to hour boundary in create_batch_hash: {}",
                start_timestamp
            );
        }

        const MAX_RETRIES: u32 = 3;
        const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

        let selector = selector!("hash_avg_fees_and_store");
        let call = Call {
            selector,
            calldata: vec![Felt::from(start_timestamp)],
            to: felt(hash_store_address)?,
        };

        let mut attempt = 0;
        loop {
            debug!(
                hash_store_address = %hash_store_address,
                start_timestamp,
                attempt = attempt + 1,
                "Creating batch hash for 180 avg fees"
            );

            match self.account.execute_v3(vec![call.clone()]).send().await {
                Ok(tx) => {
                    info!(
                        tx_hash = ?tx.transaction_hash,
                        start_timestamp,
                        "Batch hash creation successful"
                    );
                    return Ok(tx.transaction_hash);
                }
                Err(e) => {
                    if attempt >= MAX_RETRIES {
                        warn!("Max retries reached for batch hash creation");
                        return Err(e.into());
                    }

                    let backoff = INITIAL_BACKOFF * 2u32.pow(attempt);
                    warn!(
                        error = ?e,
                        retry_in = ?backoff,
                        "Batch hash creation failed, retrying..."
                    );

                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
            }
        }
    }

    /// Creates final batched hash from 32 individual batch hashes
    #[instrument(skip(self), level = "debug")]
    pub async fn create_batched_hash(
        &self,
        hash_store_address: &str,
        start_timestamp: u64,
    ) -> Result<Felt> {
        // Normalize timestamp to hour boundary (round down to nearest 3600)
        let (start_timestamp, was_normalized) = normalize_timestamp(start_timestamp);

        if was_normalized {
            warn!(
                "Timestamp normalized to hour boundary in create_batched_hash: {}",
                start_timestamp
            );
        }

        const MAX_RETRIES: u32 = 3;
        const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

        let selector = selector!("hash_batched_avg_fees");
        let call = Call {
            selector,
            calldata: vec![Felt::from(start_timestamp)],
            to: felt(hash_store_address)?,
        };

        let mut attempt = 0;
        loop {
            debug!(
                hash_store_address = %hash_store_address,
                start_timestamp,
                attempt = attempt + 1,
                "Creating final batched hash from 32 batch hashes"
            );

            match self.account.execute_v3(vec![call.clone()]).send().await {
                Ok(tx) => {
                    info!(
                        tx_hash = ?tx.transaction_hash,
                        start_timestamp,
                        "Final batched hash creation successful"
                    );
                    return Ok(tx.transaction_hash);
                }
                Err(e) => {
                    if attempt >= MAX_RETRIES {
                        warn!("Max retries reached for batched hash creation");
                        return Err(e.into());
                    }

                    let backoff = INITIAL_BACKOFF * 2u32.pow(attempt);
                    warn!(
                        error = ?e,
                        retry_in = ?backoff,
                        "Batched hash creation failed, retrying..."
                    );

                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
            }
        }
    }

    /// Verifies Groth16 proof onchain (for PITCHLAKE_VERIFIER_CONTRACT)
    #[instrument(skip(self, proof), level = "debug")]
    pub async fn verify_proof(
        &self,
        verifier_address: &str,
        proof: Vec<Felt>,
        pitchlake_job_request: PitchLakeJobRequest,
    ) -> Result<Felt> {
        const MAX_RETRIES: u32 = 3;
        const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

        // Encode parameters using starknet-rs encoding
        let mut calldata = vec![];
        let proof_length = proof.len();

        // Encode the proof Vec<Felt> -> [length, ...elements] for Span<felt252>
        proof.encode(&mut calldata)?;

        // Encode the PitchLakeJobRequest struct -> [field1, field2, field3]
        pitchlake_job_request.encode(&mut calldata)?;

        debug!(
            "Calldata constructed: proof_length={}, total_calldata_length={}",
            proof_length,
            calldata.len()
        );

        let selector =
            Felt::from_hex("0x821b8b00fd9e4b2b57538b4571c0227e80f5dbdbfef0628722b3f06f3188")
                .unwrap();
        let call = Call {
            selector,
            calldata,
            to: felt(verifier_address)?,
        };

        let mut attempt = 0;
        loop {
            debug!(
                verifier_address = %verifier_address,
                proof_length = proof_length,
                vault_address = ?pitchlake_job_request.vault_address,
                timestamp = pitchlake_job_request.timestamp,
                program_id = ?pitchlake_job_request.program_id,
                attempt = attempt + 1,
                "Verifying proof onchain with PitchLakeJobRequest"
            );

            match self.account.execute_v3(vec![call.clone()]).send().await {
                Ok(tx) => {
                    info!(
                        tx_hash = ?tx.transaction_hash,
                        "Proof onchain verification successful"
                    );
                    return Ok(tx.transaction_hash);
                }
                Err(e) => {
                    if attempt >= MAX_RETRIES {
                        warn!("Max retries reached for proof verification");
                        return Err(e.into());
                    }

                    let backoff = INITIAL_BACKOFF * 2u32.pow(attempt);
                    warn!(
                        error = ?e,
                        retry_in = ?backoff,
                        "Proof verification failed, retrying..."
                    );

                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    // Helper function to create a test provider
    fn create_test_provider() -> Arc<JsonRpcClient<HttpTransport>> {
        Arc::new(JsonRpcClient::new(HttpTransport::new(
            url::Url::parse("http://localhost:5050").unwrap(),
        )))
    }

    #[test]
    fn test_new_account_success() {
        let provider = create_test_provider();
        let private_key = "0x1234567890abcdef";
        let address = "0x987654321fedcba";

        let result = StarknetAccount::new(provider, private_key, address);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_account_invalid_private_key() {
        let provider = create_test_provider();
        let private_key = "invalid_key";
        let address = "0x987654321fedcba";

        let result = StarknetAccount::new(provider, private_key, address);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_account_invalid_address() {
        let provider = create_test_provider();
        let private_key = "0x1234567890abcdef";
        let address = "invalid_address";

        let result = StarknetAccount::new(provider, private_key, address);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_batch_hash() {
        let provider = create_test_provider();
        let account =
            StarknetAccount::new(provider, "0x1234567890abcdef", "0x987654321fedcba").unwrap();

        let hash_store_address = "0x123456789";
        let start_timestamp = 1672531200u64;

        // Note: This test will fail in real execution since we're using a dummy provider
        let result = account
            .create_batch_hash(hash_store_address, start_timestamp)
            .await;
        assert!(result.is_err()); // Will error due to dummy provider
    }

    #[tokio::test]
    async fn test_verify_proof() {
        let provider = create_test_provider();
        let account =
            StarknetAccount::new(provider, "0x1234567890abcdef", "0x987654321fedcba").unwrap();

        let verifier_address = "0x123456789";
        let proof = vec![Felt::from_str("0x1").unwrap()];
        let job_request = super::PitchLakeJobRequest {
            vault_address: Felt::from_str("0x123").unwrap(),
            timestamp: 1672531200u64,
            program_id: Felt::from_str("0x504954434c4c414b455f5631").unwrap(), // 'PITCH_LAKE_V1'
        };

        // Note: This test will fail in real execution since we're using a dummy provider
        let result = account
            .verify_proof(verifier_address, proof, job_request)
            .await;
        assert!(result.is_err()); // Will error due to dummy provider
    }

    #[test]
    fn test_felt_helper() {
        // Test valid hex string
        let result = felt("0x123456789abcdef");
        assert!(result.is_ok());

        // Test invalid hex string
        let result = felt("invalid_hex");
        assert!(result.is_err());

        // Test empty string
        let result = felt("");
        assert!(result.is_err());
    }
}
