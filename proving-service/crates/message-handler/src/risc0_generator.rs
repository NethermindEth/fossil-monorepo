use async_trait::async_trait;
use eyre::{Result, eyre};
#[cfg(feature = "mock-proof")]
use std::time::Instant;
#[cfg(feature = "mock-proof")]
use tracing::{debug, warn};
use tracing::{error, info, instrument};

use risc0_zkvm::Receipt;
use starknet_crypto::Felt;

#[cfg(feature = "mock-proof")]
use coprocessor_core::{ProofCompositionInput, ProofCompositionOutput};
#[cfg(feature = "mock-proof")]
use garaga_rs::{
    calldata::full_proof_with_hints::groth16::{
        Groth16Proof, get_groth16_calldata_felt, risc0_utils::get_risc0_vk,
    },
    definitions::CurveID,
};
#[cfg(feature = "mock-proof")]
use mock_proof_composition_methods::MOCK_PROOF_COMPOSITION_GUEST_ELF;
#[cfg(feature = "mock-proof")]
use nalgebra::DVector;
#[cfg(feature = "mock-proof")]
use risc0_ethereum_contracts::encode_seal;
#[cfg(feature = "mock-proof")]
use risc0_zkvm::{ExecutorEnv, ProverOpts, VerifierContext, compute_image_id, default_prover};

use crate::proof_composition::ProofTimestampRanges;

/// Configuration for RISC0 proof generation
#[derive(Debug, Clone)]
pub struct Risc0Config {
    pub max_retries: u32,
    pub initial_retry_delay_ms: u64,
    pub verifier_contract_address: String,
}

impl Default for Risc0Config {
    fn default() -> Self {
        Self {
            max_retries: std::env::var("RISC0_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5), // Increased default retries for Bonsai API
            initial_retry_delay_ms: std::env::var("RISC0_INITIAL_RETRY_DELAY_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2000), // Increased initial delay for Bonsai API
            verifier_contract_address: std::env::var("PITCHLAKE_VERIFIER_CONTRACT")
                .unwrap_or_else(|_| "0x0".to_string()),
        }
    }
}

/// RISC0 proof generation result
#[derive(Debug, Clone)]
pub struct Risc0ProofResult {
    pub receipt: Receipt,
    pub calldata: Vec<Felt>,
    #[cfg(feature = "mock-proof")]
    pub journal_output: ProofCompositionOutput,
    pub generation_time_ms: u64,
}

/// Trait for RISC0 proof generators
#[async_trait]
pub trait Risc0ProofGenerator {
    /// Generate proof using actual fee data from the hash store
    async fn generate_proof_with_data(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        config: Risc0Config,
    ) -> Result<Risc0ProofResult>;

    /// Generate mock proof for testing (feature-gated)
    #[cfg(feature = "mock-proof")]
    async fn generate_mock_proof(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        config: Risc0Config,
    ) -> Result<Risc0ProofResult>;
}

/// Production RISC0 proof generator
#[derive(Debug, Default)]
pub struct Risc0Generator;

impl Risc0Generator {
    pub const fn new() -> Self {
        Self
    }

    #[cfg(feature = "mock-proof")]
    #[instrument(skip(self), level = "debug")]
    async fn generate_proof_internal(
        &self,
        input: ProofCompositionInput,
        config: &Risc0Config,
    ) -> Result<Risc0ProofResult> {
        let start_time = Instant::now();

        debug!(
            "Starting RISC0 proof generation with input: start_timestamp={}, end_timestamp={}, data_points={}",
            input.start_timestamp,
            input.end_timestamp,
            input.data_8_months.len()
        );

        let mut last_error = None;

        for attempt in 1..=config.max_retries {
            info!(
                "🔄 RISC0 proof generation attempt {}/{} starting (Bonsai API call)",
                attempt, config.max_retries
            );
            debug!(
                "Attempt details: delay_ms={}, total_elapsed={:?}",
                config.initial_retry_delay_ms * (2_u64.pow(attempt - 1)),
                start_time.elapsed()
            );

            match self.generate_proof_attempt(input.clone()).await {
                Ok(result) => {
                    let generation_time_ms = start_time.elapsed().as_millis() as u64;

                    info!(
                        "RISC0 proof generation successful on attempt {}/{}, took {}ms",
                        attempt, config.max_retries, generation_time_ms
                    );

                    return Ok(Risc0ProofResult {
                        receipt: result.0,
                        calldata: result.1,
                        journal_output: result.2,
                        generation_time_ms,
                    });
                }
                Err(e) => {
                    error!(
                        "RISC0 proof generation attempt {}/{} failed: {}",
                        attempt, config.max_retries, e
                    );

                    last_error = Some(e);

                    if attempt < config.max_retries {
                        let delay = config.initial_retry_delay_ms * (2_u64.pow(attempt - 1));
                        warn!("Retrying RISC0 proof generation in {}ms", delay);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        let final_error = last_error.unwrap_or_else(|| {
            eyre!(
                "RISC0 proof generation failed after {} attempts with no error details",
                config.max_retries
            )
        });

        error!(
            "RISC0 proof generation failed after all {} attempts: {}",
            config.max_retries, final_error
        );

        Err(final_error)
    }

    #[cfg(feature = "mock-proof")]
    #[instrument(skip(self, input), level = "debug")]
    async fn generate_proof_attempt(
        &self,
        input: ProofCompositionInput,
    ) -> Result<(Receipt, Vec<Felt>, ProofCompositionOutput)> {
        debug!("Creating executor environment for RISC0 proof");

        // Check Bonsai API environment variables
        let bonsai_api_key =
            std::env::var("BONSAI_API_KEY").unwrap_or_else(|_| "NOT_SET".to_string());
        let bonsai_api_url =
            std::env::var("BONSAI_API_URL").unwrap_or_else(|_| "NOT_SET".to_string());
        debug!(
            "🔐 Bonsai API Key: {} (length: {})",
            if bonsai_api_key == "NOT_SET" {
                "NOT_SET"
            } else {
                "***REDACTED***"
            },
            bonsai_api_key.len()
        );
        debug!("🌐 Bonsai API URL: {}", bonsai_api_url);

        info!("🚀 Starting RISC0 proof generation with Groth16 via Bonsai API");
        info!("📡 Calling Bonsai prover service for mock proof composition...");

        let start_time = std::time::Instant::now();
        info!("⏰ Proof generation start time: {:?}", start_time);

        // Use spawn_blocking to avoid tokio runtime conflicts (following fossil-light-client pattern)
        let result = {
            let prover_result = match tokio::task::spawn_blocking({
                let input = input.clone();

                move || -> Result<risc0_zkvm::ProveInfo> {
                    debug!("Building executor environment with input data");
                    let env = ExecutorEnv::builder()
                        .write(&input)
                        .map_err(|e| {
                            eyre!("Failed to write input data to executor environment: {}", e)
                        })?
                        .build()
                        .map_err(|e| eyre!("Failed to build executor environment: {}", e))?;

                    // Enhanced logging for proof generation with Bonsai-specific retry handling
                    default_prover()
                        .prove_with_ctx(
                            env,
                            &VerifierContext::default(),
                            MOCK_PROOF_COMPOSITION_GUEST_ELF,
                            &ProverOpts::groth16(),
                        )
                        .map_err(|e| eyre!("RISC0 proof generation failed: {}", e))
                }
            })
            .await
            {
                Ok(blocking_result) => match blocking_result {
                    Ok(result) => {
                        let elapsed = start_time.elapsed();
                        info!("✅ Bonsai proof generation successful in {:?}", elapsed);
                        result
                    }
                    Err(e) => {
                        let elapsed = start_time.elapsed();
                        error!(
                            "❌ Bonsai proof generation failed after {:?}: {}",
                            elapsed, e
                        );

                        // Enhanced error diagnostics
                        if e.to_string().contains("HTTP") {
                            error!("🌐 HTTP Error Details:");
                            error!(
                                "   - This appears to be a network connectivity issue with Bonsai API"
                            );
                            error!("   - Check internet connection and Bonsai API status");
                            error!("   - API URL: {}", bonsai_api_url);
                            if bonsai_api_key == "NOT_SET" {
                                error!("   - ⚠️  BONSAI_API_KEY is not set!");
                            }
                        } else if e.to_string().contains("timeout")
                            || e.to_string().contains("Timeout")
                        {
                            error!("⏱️  Timeout Error Details:");
                            error!("   - Bonsai API request timed out");
                            error!("   - This may be due to high API load or network latency");
                            error!("   - Consider retrying the operation");
                        } else if e.to_string().contains("authentication")
                            || e.to_string().contains("auth")
                        {
                            error!("🔐 Authentication Error Details:");
                            error!("   - Check if BONSAI_API_KEY is valid and not expired");
                            error!("   - API Key length: {}", bonsai_api_key.len());
                        } else {
                            error!("🔧 General Error Details:");
                            error!("   - Error source: RISC0 proof generation");
                            error!("   - Full error: {:#}", e);
                        }

                        return Err(e);
                    }
                },
                Err(join_error) => {
                    let elapsed = start_time.elapsed();
                    error!(
                        "❌ RISC0 proof generation task failed to complete after {:?}: {}",
                        elapsed, join_error
                    );
                    return Err(eyre!(
                        "RISC0 proof generation task panicked: {}",
                        join_error
                    ));
                }
            };

            let elapsed = start_time.elapsed();
            info!("⏱️  Bonsai proof generation completed in {:?}", elapsed);

            let receipt = prover_result.receipt;
            debug!("RISC0 proof generation completed, processing receipt");

            // Decode the journal to get the output
            let journal_output: ProofCompositionOutput = receipt
                .journal
                .decode()
                .map_err(|e| eyre!("Failed to decode journal output: {}", e))?;

            debug!(
                "Journal decoded successfully: reserve_price={}, twap_result={}, max_return={}",
                journal_output.reserve_price, journal_output.twap_result, journal_output.max_return
            );

            debug!("Encoding seal for Groth16 proof");
            let encoded_seal =
                encode_seal(&receipt).map_err(|e| eyre!("Failed to encode seal: {}", e))?;

            debug!("Computing image ID");
            let image_id = compute_image_id(MOCK_PROOF_COMPOSITION_GUEST_ELF)
                .map_err(|e| eyre!("Failed to compute image ID: {}", e))?;

            let journal = receipt.journal.bytes.clone();
            debug!("Journal bytes length: {}", journal.len());

            info!("🔧 Creating Groth16 proof from RISC0 receipt for StarkNet verification");
            let groth16_proof =
                Groth16Proof::from_risc0(encoded_seal, image_id.as_bytes().to_vec(), journal);

            info!("⚙️  Generating StarkNet calldata for onchain verification...");
            let calldata =
                get_groth16_calldata_felt(&groth16_proof, &get_risc0_vk(), CurveID::BN254)
                    .map_err(|e| eyre!("Failed to generate calldata: {}", e))?;

            // Log the proof calldata for verification that Bonsai integration is working
            info!(
                "🎉 Successfully generated RISC0 proof calldata! (length: {})",
                calldata.len()
            );
            debug!(
                "🔍 Generated RISC0 proof calldata (length: {}): {:?}",
                calldata.len(),
                calldata
            );
            info!(
                "✅ RISC0 proof processing completed successfully, calldata length: {}",
                calldata.len()
            );

            Ok((receipt, calldata, journal_output))
        };

        result
    }

    #[cfg(feature = "mock-proof")]
    fn create_mock_input(timestamp_ranges: &ProofTimestampRanges) -> ProofCompositionInput {
        debug!("Creating mock input data for proof generation");

        // Create mock data that matches the expected structure
        ProofCompositionInput {
            data_8_months: vec![0.1, 0.2, 0.3, 0.4, 0.5], // Mock fee data
            data_8_months_hash: [
                0x12345678, 0x23456789, 0x3456789a, 0x456789ab, 0x56789abc, 0x6789abcd, 0x789abcde,
                0x89abcdef,
            ], // Mock hash of fee data
            start_timestamp: timestamp_ranges.overall_range().0,
            end_timestamp: timestamp_ranges.overall_range().1,
            positions: vec![1.0, 2.0, 3.0, 4.0, 5.0], // Mock positions
            pt: DVector::from_vec(vec![0.1, 0.2, 0.3]), // Mock statistical data
            pt_1: DVector::from_vec(vec![0.2, 0.3, 0.4]),
            gradient_tolerance: 0.001,
            de_seasonalised_detrended_log_base_fee: DVector::from_vec(vec![0.5, 0.6, 0.7]),
            n_periods: 24,
            num_paths: 100,
            season_param: DVector::from_vec(vec![0.8, 0.9, 1.0]),
            twap_7d: vec![1.1, 1.2, 1.3],
            slope: 0.05,
            intercept: 1.5,
            reserve_price: 2.5,
            floating_point_tolerance: 0.0001,
            reserve_price_tolerance: 0.01,
            twap_tolerance: 0.05,
            twap_result: 1.25,
            max_return: 0.3,
        }
    }
}

#[async_trait]
impl Risc0ProofGenerator for Risc0Generator {
    #[instrument(skip(self), level = "info")]
    #[allow(unused_variables)]
    async fn generate_proof_with_data(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        config: Risc0Config,
    ) -> Result<Risc0ProofResult> {
        info!(
            "Starting proof generation with real data for ranges: {:?}",
            timestamp_ranges
        );

        // TODO: Implement real data fetching and proof generation
        // This will integrate with the hash store and use real fee data
        #[cfg(not(feature = "mock-proof"))]
        {
            error!("Real proof generation not yet implemented - mock-proof feature required");
            return Err(eyre!(
                "Real proof generation with data is not yet implemented. Use mock-proof feature for testing."
            ));
        }

        #[cfg(feature = "mock-proof")]
        {
            warn!("Using mock proof generation - real data integration not yet implemented");
            self.generate_mock_proof(timestamp_ranges, config).await
        }
    }

    #[cfg(feature = "mock-proof")]
    #[instrument(skip(self), level = "info")]
    async fn generate_mock_proof(
        &self,
        timestamp_ranges: ProofTimestampRanges,
        config: Risc0Config,
    ) -> Result<Risc0ProofResult> {
        info!(
            "Starting mock proof generation for ranges: {:?}",
            timestamp_ranges
        );

        let input = Self::create_mock_input(&timestamp_ranges);

        debug!(
            "Mock input created: data_length={}, start={}, end={}",
            input.data_8_months.len(),
            input.start_timestamp,
            input.end_timestamp
        );

        let result = self.generate_proof_internal(input, &config).await?;

        info!(
            "Mock proof generation completed successfully in {}ms",
            result.generation_time_ms
        );

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risc0_config_default() {
        let config = Risc0Config::default();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_retry_delay_ms, 2000);
    }

    #[test]
    fn test_risc0_generator_new() {
        let generator = Risc0Generator::new();
        // Just test that it can be created
        assert_eq!(format!("{:?}", generator), "Risc0Generator");
    }

    #[cfg(feature = "mock-proof")]
    #[test]
    fn test_create_mock_input() {
        let ranges = ProofTimestampRanges::new(
            1672531200, 1672617600, // twap
            1672531200, 1672617600, // reserve_price
            1672531200, 1672617600, // max_return
        );

        let input = Risc0Generator::create_mock_input(&ranges);
        assert_eq!(input.start_timestamp, 1672531200);
        assert_eq!(input.end_timestamp, 1672617600);
        assert!(!input.data_8_months.is_empty());
    }

    #[cfg(feature = "mock-proof")]
    #[tokio::test]
    async fn test_generate_mock_proof_integration() {
        // This test would require setting up a test environment
        // with proper RISC0 dependencies. For now, we'll skip it
        // in the actual test run but provide the structure.

        /*
        let generator = Risc0Generator::new();
        let config = Risc0Config::default();
        let ranges = ProofTimestampRanges::new(
            1672531200, 1672617600,
            1672531200, 1672617600,
            1672531200, 1672617600,
        );

        let result = generator.generate_mock_proof(ranges, config).await;
        assert!(result.is_ok());
        */
    }
}
