#[cfg(feature = "proof-composition")]
use add_twap_7d_error_bound_floating::add_twap_7d_error_bound;
#[cfg(feature = "proof-composition")]
use calculate_pt_pt1_error_bound_floating::calculate_pt_pt1_error_bound_floating;
#[cfg(feature = "proof-composition")]
use coprocessor_common::{
    floating_point,
    original::{self, convert_array1_to_dvec},
    tests::mock::convert_data_to_vec_of_tuples,
};
#[cfg(feature = "mock-proof")]
use coprocessor_core::ProofCompositionInput;
#[cfg(feature = "proof-composition")]
use coprocessor_core::{
    AddTwap7dErrorBoundFloatingInput, CalculatePtPt1ErrorBoundFloatingInput, HashingFeltInput,
    MaxReturnInput, ProofCompositionInput, RemoveSeasonalityErrorBoundFloatingInput,
    SimulatePriceVerifyPositionInput, TwapErrorBoundInput,
};
use eyre::{Result, eyre};
#[cfg(feature = "proof-composition")]
use hashing_felts::hash_felts;
#[cfg(feature = "proof-composition")]
use max_return_floating::max_return;
#[cfg(feature = "mock-proof")]
use mock_proof_composition_methods::MOCK_PROOF_COMPOSITION_GUEST_ELF;
#[cfg(feature = "proof-composition")]
use proof_composition_twap_maxreturn_reserveprice_floating_hashing_methods::PROOF_COMPOSITION_TWAP_MAXRETURN_RESERVEPRICE_FLOATING_HASHING_GUEST_ELF;
#[cfg(feature = "proof-composition")]
use remove_seasonality_error_bound_floating::remove_seasonality_error_bound;
#[cfg(any(not(feature = "proof-composition"), feature = "mock-proof"))]
use risc0_zkvm::Receipt;
#[cfg(feature = "mock-proof")]
use risc0_zkvm::{ExecutorEnv, ProverOpts, VerifierContext, default_prover};
#[cfg(feature = "proof-composition")]
use risc0_zkvm::{ExecutorEnv, Receipt, default_prover};
#[cfg(feature = "proof-composition")]
use simulate_price_verify_position_floating::simulate_price_verify_position;
#[cfg(feature = "proof-composition")]
use starknet::core::types::Felt;
use std::cmp::{max, min};
#[cfg(feature = "proof-composition")]
use tokio::{task, try_join};
#[cfg(feature = "proof-composition")]
use twap_error_bound_floating::calculate_twap;

#[cfg(feature = "mock-proof")]
use nalgebra::DVector;

/// Struct to hold different timestamp ranges for proof calculations
#[derive(Debug, Clone)]
pub struct ProofTimestampRanges {
    pub twap: (i64, i64),
    pub reserve_price: (i64, i64),
    pub max_return: (i64, i64),
}

impl ProofTimestampRanges {
    pub const fn new(
        twap_start: i64,
        twap_end: i64,
        reserve_price_start: i64,
        reserve_price_end: i64,
        max_return_start: i64,
        max_return_end: i64,
    ) -> Self {
        Self {
            twap: (twap_start, twap_end),
            reserve_price: (reserve_price_start, reserve_price_end),
            max_return: (max_return_start, max_return_end),
        }
    }

    /// Returns the overall start and end timestamps covering all calculations
    pub fn overall_range(&self) -> (i64, i64) {
        let start = min(min(self.twap.0, self.reserve_price.0), self.max_return.0);
        let end = max(max(self.twap.1, self.reserve_price.1), self.max_return.1);
        (start, end)
    }
}

#[async_trait::async_trait]
pub trait ProofProvider {
    // TODO: separate composition from generation
    // TODO: add error handling

    async fn generate_proofs_from_data(
        &self,
        timestamp_ranges: ProofTimestampRanges,
    ) -> Result<Receipt>;

    fn is_disabled(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub struct BonsaiProofProvider;

impl BonsaiProofProvider {
    pub const fn new() -> Self {
        Self
    }

    pub const fn is_disabled(&self) -> bool {
        false
    }
}

impl Default for BonsaiProofProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ProofProvider for BonsaiProofProvider {
    #[allow(unused_variables)]
    async fn generate_proofs_from_data(
        &self,
        timestamp_ranges: ProofTimestampRanges,
    ) -> Result<Receipt> {
        #[cfg(feature = "proof-composition")]
        {
            use crate::hashing::HashingProvider;
            use crate::services::hashing_service::HashingService;
            use starknet_handler::{config::load_starknet_config, provider::StarknetProvider};

            // Initialize StarkNet provider
            let config = load_starknet_config()?;
            let provider = StarknetProvider::new(config)?;

            // Initialize HashingService for hash availability validation
            let hashing_provider = HashingProvider::from_env()
                .map_err(|e| eyre!("Failed to initialize HashingProvider: {}", e))?;
            let hashing_service = HashingService::new(hashing_provider, 5760, 180);

            // Get the overall range covering all calculations for fee fetching
            let (overall_start, overall_end) = timestamp_ranges.overall_range();

            // Run hash preparation before proof generation
            tracing::info!(
                "🔧 Running hash preparation for timestamp range: {} to {}",
                overall_start,
                overall_end
            );
            hashing_service
                .run(overall_start as u64)
                .await
                .map_err(|e| eyre!("Hash preparation failed: {}", e))?;
            tracing::info!("✅ Hash preparation completed successfully");

            // Fetch fees using the new starknet-handler
            let fee_data = provider
                .get_avg_fees_in_range(overall_start as u64, overall_end as u64)
                .await?;

            // The system expects exactly 5760 fee values (8 months of hourly data)
            // All fees must come from onchain - no padding or artificial generation!

            // Validate that we have sufficient onchain data
            if fee_data.block_hashes.len() < 5760 {
                return Err(eyre!(
                    "Insufficient onchain fee data: got {} values, need exactly 5760 (8 months of hourly data). \
                Time range: {} to {}. Please ensure the fossil_store contract has sufficient historical data.",
                    fee_data.block_hashes.len(),
                    overall_start,
                    overall_end
                ));
            }

            // Convert the first 5760 block hashes to the format expected by the hashing system
            let raw_input: Vec<String> = fee_data
                .block_hashes
                .iter()
                .take(5760) // Take exactly 5760 values
                .cloned()
                .collect();

            // Validate that all hashes are valid hex strings
            for (i, hash) in raw_input.iter().enumerate() {
                if Felt::from_hex(hash).is_err() {
                    return Err(eyre!(
                        "Invalid hex hash at index {}: '{}'. All onchain fee data must be valid hex values.",
                        i,
                        hash
                    ));
                }
            }

            // Convert onchain fee data to felts for hashing
            let mut res = Vec::with_capacity(5760);
            for hash_str in &raw_input {
                let felt = Felt::from_hex(hash_str).map_err(|e| {
                    eyre!(
                        "Failed to convert onchain fee data '{}' to Felt: {}",
                        hash_str,
                        e
                    )
                })?;
                res.push(felt);
            }
            let (hashing_receipt, hashing_res) = hash_felts(HashingFeltInput { inputs: res });

            let data_8_months = hashing_res.f64_inputs;
            let data = data_8_months[data_8_months.len().saturating_sub(2160)..].to_vec();

            // Extract specific timestamp ranges for each calculation
            let (twap_start, twap_end) = timestamp_ranges.twap;
            let (reserve_price_start, reserve_price_end) = timestamp_ranges.reserve_price;
            let (max_return_start, max_return_end) = timestamp_ranges.max_return;

            // max return
            let input = MaxReturnInput { data: data.clone() };
            let (max_return_receipt, max_return_res) = max_return(input);

            // twap
            // replacing  original::calculate_twap::calculate_twap with this, as we are using random avg fee hourly data
            // that we dont have the underlying raw data for
            let twap_original = floating_point::calculate_twap(&data);
            let input = TwapErrorBoundInput {
                avg_hourly_gas_fee: data.clone(),
                twap_tolerance: 1.0,
                twap_result: twap_original,
            };

            let (calculate_twap_receipt, _calculate_twap_res) = calculate_twap(input);

            // reserve price
            // run rust code in host
            // ensure convergence in host
            let n_periods = 720;

            // Use reserve price specific range for data with timestamps
            let data_with_timestamps =
                convert_data_to_vec_of_tuples(data.clone(), reserve_price_start);
            let res = original::calculate_reserve_price(&data_with_timestamps, 15000, n_periods);

            let num_paths = 4000;
            let gradient_tolerance = 5e-2;
            let floating_point_tolerance = 0.00001; // 0.00001%
            let reserve_price_tolerance = 5.0; // 5%

            // Making all these async via tokio spawns

            // Remove seasonality error bound
            let remove_seasonality_error_bound_input = RemoveSeasonalityErrorBoundFloatingInput {
                data: data.clone(),
                slope: res.slope,
                intercept: res.intercept,
                de_seasonalised_detrended_log_base_fee: convert_array1_to_dvec(
                    res.de_seasonalised_detrended_log_base_fee.clone(),
                ),
                season_param: convert_array1_to_dvec(res.season_param.clone()),
                tolerance: floating_point_tolerance,
            };

            let remove_seasonality_task = tokio::spawn(async move {
                remove_seasonality_error_bound(remove_seasonality_error_bound_input)
            });

            // Calculate PT/PT1 error bound
            let calculate_pt_pt1_input = CalculatePtPt1ErrorBoundFloatingInput {
                de_seasonalised_detrended_log_base_fee: convert_array1_to_dvec(
                    res.de_seasonalised_detrended_log_base_fee.clone(),
                ),
                pt: convert_array1_to_dvec(res.pt.clone()),
                pt_1: convert_array1_to_dvec(res.pt_1.clone()),
                tolerance: floating_point_tolerance,
            };

            let calculate_pt_pt1_task = tokio::spawn(async move {
                calculate_pt_pt1_error_bound_floating(calculate_pt_pt1_input)
            });

            // TWAP 7D error bound
            let add_twap_7d_input = AddTwap7dErrorBoundFloatingInput {
                data: data.clone(),
                twap_7d: res.twap_7d.clone(),
                tolerance: floating_point_tolerance,
            };

            let add_twap_7d_task =
                tokio::spawn(async move { add_twap_7d_error_bound(add_twap_7d_input) });

            // Simulate price verify position
            let simulate_price_input = SimulatePriceVerifyPositionInput {
                start_timestamp: overall_start,
                end_timestamp: overall_end,
                positions: res.positions.clone(),
                pt: convert_array1_to_dvec(res.pt.clone()),
                pt_1: convert_array1_to_dvec(res.pt_1.clone()),
                gradient_tolerance,
                de_seasonalised_detrended_log_base_fee: convert_array1_to_dvec(
                    res.de_seasonalised_detrended_log_base_fee.clone(),
                ),
                n_periods,
                num_paths,
                season_param: convert_array1_to_dvec(res.season_param.clone()),
                twap_7d: res.twap_7d.clone(),
                slope: res.slope,
                intercept: res.intercept,
                reserve_price: res.reserve_price,
                tolerance: floating_point_tolerance,
                data_length: 2160,
            };

            let simulate_price_task =
                tokio::spawn(async move { simulate_price_verify_position(simulate_price_input) });

            // Join all the tasks
            let receipts = match try_join!(
                remove_seasonality_task,
                calculate_pt_pt1_task,
                add_twap_7d_task,
                simulate_price_task
            ) {
                Ok(receipts) => receipts,
                Err(e) => {
                    return Err(eyre!("Failed to join tasks: {}", e));
                }
            };

            // Compose proofs
            let composition_input = ProofCompositionInput {
                data_8_months: data_8_months.clone(),
                data_8_months_hash: hashing_res.hash,
                start_timestamp: overall_start,
                end_timestamp: overall_end,
                positions: res.positions.clone(),
                pt: convert_array1_to_dvec(res.pt.clone()),
                pt_1: convert_array1_to_dvec(res.pt_1.clone()),
                gradient_tolerance,
                de_seasonalised_detrended_log_base_fee: convert_array1_to_dvec(
                    res.de_seasonalised_detrended_log_base_fee.clone(),
                ),
                n_periods,
                num_paths,
                season_param: convert_array1_to_dvec(res.season_param.clone()),
                twap_7d: res.twap_7d.clone(),
                slope: res.slope,
                intercept: res.intercept,
                reserve_price: res.reserve_price,
                floating_point_tolerance,
                reserve_price_tolerance,
                twap_tolerance: 1.0,
                twap_result: twap_original,
                max_return: max_return_res.1,
            };

            // Log ProofCompositionInput details for verification
            tracing::info!("📊 ProofCompositionInput prepared and ready for real proof method:");
            tracing::info!(
                "   • data_8_months: {} values (need 5760) ✓",
                composition_input.data_8_months.len()
            );
            tracing::info!(
                "   • data_8_months_hash: {:?}",
                composition_input.data_8_months_hash
            );
            tracing::info!(
                "   • timestamps: {} to {}",
                composition_input.start_timestamp,
                composition_input.end_timestamp
            );
            tracing::info!(
                "   • positions: {} values",
                composition_input.positions.len()
            );
            tracing::info!(
                "   • Statistical data: pt={}, pt_1={}, season_param={}",
                composition_input.pt.len(),
                composition_input.pt_1.len(),
                composition_input.season_param.len()
            );
            tracing::info!(
                "   • Parameters: reserve_price={}, max_return={}, twap_result={}",
                composition_input.reserve_price,
                composition_input.max_return,
                composition_input.twap_result
            );
            tracing::info!(
                "   • Tolerances: gradient={}, floating_point={}, reserve_price={}, twap={}",
                composition_input.gradient_tolerance,
                composition_input.floating_point_tolerance,
                composition_input.reserve_price_tolerance,
                composition_input.twap_tolerance
            );

            // Check if we're using mock data
            let using_mock_data = std::env::var("USE_MOCK_STARKNET_DATA")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false);

            if using_mock_data {
                tracing::info!(
                    "🔧 Using mock StarkNet data - ProofCompositionInput contains mock-derived data"
                );
                tracing::info!("💡 This input is ready to be sent to the real proof method at:");
                tracing::info!(
                    "   /home/ametel/source/pitchlake-coprocessor/methods/proof-composition-twap-maxreturn-reserveprice-floating-hashing-methods"
                );
            } else {
                tracing::info!(
                    "🌍 Using real onchain data - ProofCompositionInput contains onchain-derived data"
                );
            }

            // Optionally save ProofCompositionInput for debugging/integration testing
            let save_proof_input = std::env::var("SAVE_PROOF_COMPOSITION_INPUT")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false);

            if save_proof_input {
                tracing::info!("💾 Saving ProofCompositionInput to proof_composition_input.json");
                match serde_json::to_string_pretty(&composition_input) {
                    Ok(json_str) => {
                        if let Err(e) = std::fs::write("proof_composition_input.json", json_str) {
                            tracing::warn!("Failed to save ProofCompositionInput: {}", e);
                        } else {
                            tracing::info!("✅ ProofCompositionInput saved successfully");
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to serialize ProofCompositionInput: {}", e);
                    }
                }
            }

            // Generate the composed proof
            let receipt = task::spawn_blocking(move || {
                let env = ExecutorEnv::builder()
                    .write(&composition_input)
                    .unwrap()
                    .build()
                    .unwrap();

                default_prover().prove(
                    env,
                    PROOF_COMPOSITION_TWAP_MAXRETURN_RESERVEPRICE_FLOATING_HASHING_GUEST_ELF,
                )
            })
            .await
            .unwrap()
            .unwrap()
            .receipt;

            Ok(receipt)
        }

        #[cfg(feature = "mock-proof")]
        {
            // For mock-proof builds, use mock implementation
            tracing::warn!("Using mock proof generation - proof-composition feature not enabled");

            // Check if RISC0 integration is requested
            let use_risc0_integration_env = std::env::var("USE_RISC0_INTEGRATION");
            tracing::info!(
                "🔍 USE_RISC0_INTEGRATION environment variable: {:?}",
                use_risc0_integration_env
            );

            let use_risc0_integration = use_risc0_integration_env
                .map(|v| {
                    let lowercase = v.to_lowercase();
                    tracing::info!(
                        "🔍 USE_RISC0_INTEGRATION lowercase value: '{}', equals 'true': {}",
                        lowercase,
                        lowercase == "true"
                    );
                    lowercase == "true"
                })
                .unwrap_or(false);

            tracing::info!(
                "🔍 Final use_risc0_integration decision: {}",
                use_risc0_integration
            );

            if use_risc0_integration {
                tracing::info!("Using RISC0 integration path via generate_proof_with_risc0");
                return self.generate_proof_with_risc0(timestamp_ranges).await;
            }

            // When only mock-proof is enabled, delegate to the existing mock implementation
            tracing::info!("Using standard mock proof generation for generate_proofs_from_data");

            tracing::info!(
                "Generating combined mock proof for all components (twap, reserve_price, max_return)"
            );
            tracing::debug!("Using timestamp ranges: {:?}", timestamp_ranges);

            // Create and execute blocking task for mock proof generation
            let result = tokio::task::spawn_blocking(move || -> Result<Receipt> {
                // Create mock input data based on timestamp ranges
                let start_time = std::time::Instant::now();

                // Create mock input
                let mock_input = ProofCompositionInput {
                    data_8_months: vec![0.1, 0.2, 0.3, 0.4, 0.5], // Mock fee data
                    data_8_months_hash: [
                        0x12345678, 0x23456789, 0x3456789a, 0x456789ab, 0x56789abc, 0x6789abcd,
                        0x789abcde, 0x89abcdef,
                    ], // Mock hash of fee data
                    data_8_months_start_timestamp: timestamp_ranges.overall_range().0
                        - 8 * 30 * 24 * 3600, // 8 months before start
                    data_8_months_end_timestamp: timestamp_ranges.overall_range().0, // Up to the start of analysis period
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
                };

                tracing::debug!("Building executor environment with mock input data");
                let env = ExecutorEnv::builder()
                    .write(&mock_input)
                    .map_err(|e| {
                        eyre!(
                            "Failed to write mock input data to executor environment: {}",
                            e
                        )
                    })?
                    .build()
                    .map_err(|e| eyre!("Failed to build executor environment: {}", e))?;

                tracing::info!(
                    "🚀 Starting RISC0 mock proof generation with Groth16 via Bonsai API"
                );
                tracing::info!("📡 Calling Bonsai prover service for mock proof composition...");

                let prover_result = default_prover()
                    .prove_with_ctx(
                        env,
                        &VerifierContext::default(),
                        MOCK_PROOF_COMPOSITION_GUEST_ELF,
                        &ProverOpts::groth16(),
                    )
                    .map_err(|e| eyre!("RISC0 proof generation failed: {}", e))?;

                let elapsed = start_time.elapsed();
                tracing::info!("⏱️  Bonsai proof generation completed in {:?}", elapsed);

                let receipt = prover_result.receipt;
                tracing::debug!("RISC0 proof generation completed, processing receipt");

                Ok(receipt)
            })
            .await
            .map_err(|e| {
                eyre!(
                    "Failed to execute blocking RISC0 proof generation task: {}",
                    e
                )
            })??;

            Ok(result)
        }

        #[cfg(not(any(feature = "proof-composition", feature = "mock-proof")))]
        {
            Err(eyre!(
                "No proof generation features enabled. Enable either 'proof-composition' or 'mock-proof' feature."
            ))
        }
    }
}

impl BonsaiProofProvider {
    #[cfg(feature = "mock-proof")]
    async fn generate_proof_with_risc0(
        &self,
        timestamp_ranges: ProofTimestampRanges,
    ) -> Result<Receipt> {
        use crate::proof_verifier::{IntegratedProofVerifier, ProofVerifier, ProofVerifierConfig};
        use crate::risc0_generator::{Risc0Config, Risc0Generator};

        tracing::info!("🔧 Starting integrated RISC0 proof generation with on-chain verification");
        tracing::debug!("Timestamp ranges: {:?}", timestamp_ranges);

        // Create RISC0 generator and config
        let generator = Risc0Generator::new();
        let risc0_config = Risc0Config::default();

        // Create verifier configuration
        let verify_onchain = std::env::var("VERIFY_PROOFS_ONCHAIN")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        let verifier_contract_address = std::env::var("PITCHLAKE_VERIFIER_CONTRACT")
            .map_err(|_| eyre!("PITCHLAKE_VERIFIER_CONTRACT environment variable is required"))?;

        // Create job_request for onchain verification if needed
        let job_request = if verify_onchain
            && !verifier_contract_address.is_empty()
            && verifier_contract_address != "0x0"
        {
            use crate::response_handler::PitchLakeJobRequest;
            use starknet::core::types::Felt;

            // Get vault address from environment
            let vault_address_str = std::env::var("PITCHLAKE_VAULT").map_err(|_| {
                eyre!("PITCHLAKE_VAULT environment variable is required for onchain verification")
            })?;
            let vault_address = Felt::from_hex(&vault_address_str)
                .map_err(|e| eyre!("Invalid vault address format: {}", e))?;

            // Use end timestamp from the ranges as the result timestamp
            let timestamp = timestamp_ranges.overall_range().1 as u64;

            // Use standard program ID for PitchLake
            let program_id =
                Felt::from_hex("0x504954434c4c414b455f5631") // 'PITCH_LAKE_V1' in hex (corrected)
                    .map_err(|e| eyre!("Failed to create program ID: {}", e))?;

            tracing::info!(
                "🔗 Creating job request for onchain verification: vault={}, timestamp={}, program_id={}",
                vault_address,
                timestamp,
                program_id
            );

            Some(PitchLakeJobRequest {
                vault_address,
                timestamp,
                program_id,
            })
        } else {
            tracing::info!(
                "⚠️ Onchain verification disabled or invalid contract address, skipping job_request creation"
            );
            None
        };

        let verifier_config = ProofVerifierConfig {
            verify_onchain,
            verifier_contract_address: verifier_contract_address.clone(),
            risc0_config,
            job_request,
        };

        // Create integrated verifier
        let verifier = IntegratedProofVerifier::new(generator);

        // Generate and verify the proof
        let verification_result = verifier
            .generate_and_verify_proof(timestamp_ranges, verifier_config)
            .await
            .map_err(|e| eyre!("Integrated proof generation and verification failed: {}", e))?;

        tracing::info!("✅ Integrated RISC0 proof generation completed successfully!");

        if let Some(tx_hash) = verification_result.onchain_tx_hash {
            tracing::info!(
                "Proof verified onchain with transaction hash: {:?}",
                tx_hash
            );
        }

        #[cfg(feature = "mock-proof")]
        tracing::debug!(
            "Final proof result: reserve_price={}, twap_result={}, max_return={}",
            verification_result
                .risc0_result
                .journal_output
                .reserve_price,
            verification_result.risc0_result.journal_output.twap_result,
            verification_result.risc0_result.journal_output.max_return
        );

        #[cfg(not(feature = "mock-proof"))]
        tracing::debug!(
            "Final proof result generated successfully (journal details not available without mock-proof feature)"
        );

        Ok(verification_result.risc0_result.receipt)
    }
}
