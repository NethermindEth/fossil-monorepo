use async_trait::async_trait;
use eyre::{Result, eyre};
use std::time::Instant;
use tracing::{debug, error, info, instrument, warn};

use starknet_crypto::Felt;
#[cfg(feature = "starknet-handler")]
use starknet_handler::{config::load_starknet_config, provider::StarknetProvider};

use crate::proof_composition::ProofTimestampRanges;
use crate::response_handler;
use crate::risc0_generator::{Risc0Config, Risc0ProofGenerator, Risc0ProofResult};

/// Configuration for proof verification
#[derive(Debug, Clone, Default)]
pub struct ProofVerifierConfig {
    pub risc0_config: Risc0Config,
    pub verifier_contract_address: String,
    pub verify_onchain: bool,
    pub job_request: Option<response_handler::PitchLakeJobRequest>,
}

impl ProofVerifierConfig {
    pub fn from_env() -> Result<Self> {
        let verifier_contract_address = std::env::var("PITCHLAKE_VERIFIER_CONTRACT")
            .map_err(|_| eyre!("PITCHLAKE_VERIFIER_CONTRACT environment variable is required"))?;

        let verify_onchain = std::env::var("VERIFY_PROOFS_ONCHAIN")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        Ok(Self {
            risc0_config: Risc0Config::default(),
            verifier_contract_address,
            verify_onchain,
            job_request: None,
        })
    }
}

/// Complete proof verification result
#[derive(Debug, Clone)]
pub struct ProofVerificationResult {
    pub risc0_result: Risc0ProofResult,
    pub onchain_tx_hash: Option<Felt>,
    pub total_time_ms: u64,
}

/// Trait for complete proof verification pipeline
#[async_trait]
pub trait ProofVerifier {
    /// Generate proof and verify onchain (if configured)
    async fn generate_and_verify_proof(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        config: ProofVerifierConfig,
    ) -> Result<ProofVerificationResult>;
}

/// Production proof verifier that integrates RISC0 generation with `StarkNet` verification
#[derive(Debug)]
pub struct IntegratedProofVerifier<G: Risc0ProofGenerator> {
    generator: G,
}

impl<G: Risc0ProofGenerator> IntegratedProofVerifier<G> {
    pub const fn new(generator: G) -> Self {
        Self { generator }
    }

    #[cfg(feature = "starknet-handler")]
    #[instrument(skip(self, calldata), level = "debug")]
    async fn verify_proof_onchain(
        &self,
        calldata: Vec<Felt>,
        verifier_address: &str,
        job_request: response_handler::PitchLakeJobRequest,
    ) -> Result<Felt> {
        debug!(
            "Starting onchain proof verification with {} calldata elements",
            calldata.len()
        );

        // Load StarkNet configuration
        let config = load_starknet_config()
            .map_err(|e| eyre!("Failed to load StarkNet configuration: {}", e))?;

        debug!(
            "StarkNet configuration loaded - RPC: {}, Account configured: {}",
            config.rpc_url,
            config.account_address.is_some()
        );

        // Create provider
        let provider = StarknetProvider::new(config)
            .map_err(|e| eyre!("Failed to create StarkNet provider: {}", e))?;

        info!(
            "Submitting proof verification to contract: {}",
            verifier_address
        );

        // Convert to starknet-handler PitchLakeJobRequest
        let starknet_job_request = starknet_handler::account::PitchLakeJobRequest {
            vault_address: job_request.vault_address,
            timestamp: job_request.timestamp,
            program_id: job_request.program_id,
        };

        // Verify proof onchain
        let tx_hash = provider
            .verify_proof_onchain(verifier_address, calldata, starknet_job_request)
            .await
            .map_err(|e| eyre!("Onchain proof verification failed: {}", e))?;

        info!(
            "Proof verification submitted successfully, tx hash: {:?}",
            tx_hash
        );

        Ok(tx_hash)
    }

    #[cfg(not(feature = "starknet-handler"))]
    async fn verify_proof_onchain(
        &self,
        _calldata: Vec<Felt>,
        _verifier_address: &str,
        _job_request: response_handler::PitchLakeJobRequest,
    ) -> Result<Felt> {
        error!("Onchain verification requested but starknet-handler feature not enabled");
        Err(eyre!(
            "Onchain verification requires the 'starknet-handler' feature to be enabled"
        ))
    }
}

#[async_trait]
impl<G: Risc0ProofGenerator + Send + Sync> ProofVerifier for IntegratedProofVerifier<G> {
    #[instrument(skip(self), level = "info")]
    async fn generate_and_verify_proof(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        config: ProofVerifierConfig,
    ) -> Result<ProofVerificationResult> {
        let total_start_time = Instant::now();

        info!(
            "Starting complete proof generation and verification pipeline for ranges: {:?}",
            timestamp_ranges
        );

        debug!(
            "Configuration: onchain_verification={}, verifier_contract={}",
            config.verify_onchain, config.verifier_contract_address
        );

        // Step 1: Generate RISC0 proof
        info!("Step 1: Generating RISC0 proof");
        let risc0_result = self
            .generator
            .generate_proof_with_data(timestamp_ranges, config.risc0_config)
            .await
            .map_err(|e| eyre!("RISC0 proof generation failed: {}", e))?;

        info!(
            "RISC0 proof generated successfully in {}ms, calldata length: {}",
            risc0_result.generation_time_ms,
            risc0_result.calldata.len()
        );

        #[cfg(feature = "mock-proof")]
        debug!(
            "Proof journal output: reserve_price={}, twap_result={}, max_return={}",
            risc0_result.journal_output.reserve_price,
            risc0_result.journal_output.twap_result,
            risc0_result.journal_output.max_return
        );

        #[cfg(not(feature = "mock-proof"))]
        debug!(
            "Proof generation completed (journal details not available without mock-proof feature)"
        );

        // Step 2: Verify onchain (if configured)
        let onchain_tx_hash = if config.verify_onchain {
            info!("Step 2: Verifying proof onchain");

            if config.verifier_contract_address.is_empty()
                || config.verifier_contract_address == "0x0"
            {
                warn!("Verifier contract address is empty or 0x0, skipping onchain verification");
                None
            } else {
                // Get job request from config - required for production
                let job_request = config.job_request.clone().ok_or_else(|| {
                    eyre!("job_request is required in config for onchain verification")
                })?;

                match self
                    .verify_proof_onchain(
                        risc0_result.calldata.clone(),
                        &config.verifier_contract_address,
                        job_request,
                    )
                    .await
                {
                    Ok(tx_hash) => {
                        info!("Onchain verification completed successfully");
                        Some(tx_hash)
                    }
                    Err(e) => {
                        error!("Onchain verification failed: {}", e);
                        return Err(e);
                    }
                }
            }
        } else {
            info!("Step 2: Skipping onchain verification (disabled in config)");
            None
        };

        let total_time_ms = total_start_time.elapsed().as_millis() as u64;

        info!(
            "Complete proof pipeline finished successfully in {}ms (RISC0: {}ms)",
            total_time_ms, risc0_result.generation_time_ms
        );

        Ok(ProofVerificationResult {
            risc0_result,
            onchain_tx_hash,
            total_time_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risc0_generator::{Risc0Generator, Risc0ProofResult};
    use async_trait::async_trait;

    // Mock generator for testing
    struct MockRisc0Generator;

    #[async_trait]
    impl Risc0ProofGenerator for MockRisc0Generator {
        async fn generate_proof_with_data(
            &self,
            _timestamp_ranges: ProofTimestampRanges,
            _config: Risc0Config,
        ) -> Result<Risc0ProofResult> {
            Err(eyre!("Mock generator always fails"))
        }

        #[cfg(feature = "mock-proof")]
        async fn generate_mock_proof(
            &self,
            _timestamp_ranges: ProofTimestampRanges,
            _config: Risc0Config,
        ) -> Result<Risc0ProofResult> {
            Err(eyre!("Mock generator always fails"))
        }
    }

    #[test]
    fn test_proof_verifier_config_default() {
        let config = ProofVerifierConfig::default();
        assert!(!config.verify_onchain); // Default should be false
        assert_eq!(config.verifier_contract_address, ""); // Default is now empty string
        assert!(config.job_request.is_none());
    }

    #[test]
    fn test_integrated_proof_verifier_creation() {
        let generator = Risc0Generator::new();
        let verifier = IntegratedProofVerifier::new(generator);

        // Just test that it can be created
        assert!(format!("{:?}", verifier).contains("IntegratedProofVerifier"));
    }

    #[tokio::test]
    async fn test_generate_and_verify_proof_with_mock_failure() {
        let mock_generator = MockRisc0Generator;
        let verifier = IntegratedProofVerifier::new(mock_generator);
        let config = ProofVerifierConfig::default();

        let ranges = ProofTimestampRanges::new(
            1672531200, 1672617600, 1672531200, 1672617600, 1672531200, 1672617600,
        );

        let result = verifier.generate_and_verify_proof(ranges, config).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mock generator always fails")
        );
    }
}
