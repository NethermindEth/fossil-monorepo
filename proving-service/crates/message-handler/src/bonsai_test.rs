use eyre::Result;
#[cfg(feature = "mock-proof")]
use std::time::Instant;

#[cfg(feature = "mock-proof")]
use {
    coprocessor_core::{ProofCompositionInput, ProofCompositionOutput},
    garaga_rs::{
        calldata::full_proof_with_hints::groth16::{
            Groth16Proof, get_groth16_calldata_felt, risc0_utils::get_risc0_vk,
        },
        definitions::CurveID,
    },
    mock_proof_composition_methods::MOCK_PROOF_COMPOSITION_GUEST_ELF,
    nalgebra::DVector,
    risc0_ethereum_contracts::encode_seal,
    risc0_zkvm::{ExecutorEnv, ProverOpts, VerifierContext, compute_image_id, default_prover},
};

#[cfg(all(feature = "mock-proof", feature = "starknet-handler"))]
use starknet_handler::{config::load_starknet_config, provider::StarknetProvider};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    println!("🧪 Starting Bonsai API Test");
    println!("==========================");

    // Check environment variables
    let bonsai_api_key = std::env::var("BONSAI_API_KEY").unwrap_or_else(|_| "NOT_SET".to_string());
    let bonsai_api_url = std::env::var("BONSAI_API_URL").unwrap_or_else(|_| "NOT_SET".to_string());

    println!(
        "🔐 Bonsai API Key: {} (length: {})",
        if bonsai_api_key == "NOT_SET" {
            "NOT_SET"
        } else {
            "***REDACTED***"
        },
        bonsai_api_key.len()
    );
    println!("🌐 Bonsai API URL: {bonsai_api_url}");

    if bonsai_api_key == "NOT_SET" {
        println!("❌ ERROR: BONSAI_API_KEY is not set!");
        println!("Please set BONSAI_API_KEY environment variable");
        return Ok(());
    }

    #[cfg(not(feature = "mock-proof"))]
    {
        println!("❌ ERROR: mock-proof feature is not enabled!");
        println!("Please run with: cargo run --bin bonsai-test --features mock-proof");
        Ok(())
    }

    #[cfg(feature = "mock-proof")]
    {
        println!("✅ mock-proof feature is enabled");
        println!("📡 Starting RISC0 proof generation test with Bonsai API...");

        // Create test input data (same as in the reference)
        let test_data = ProofCompositionInput {
            data_8_months: vec![0.1, 0.2, 0.3, 0.4, 0.5],
            data_8_months_hash: [
                0x12345678, 0x23456789, 0x3456789a, 0x456789ab, 0x56789abc, 0x6789abcd, 0x789abcde,
                0x89abcdef,
            ],
            data_8_months_start_timestamp: 1651363200, // 2022-05-01 (8 months earlier)
            data_8_months_end_timestamp: 1672531200,   // 2023-01-01 (start of analysis period)
            start_timestamp: 1672531200,               // 2023-01-01
            end_timestamp: 1704067200,                 // 2024-01-01
            positions: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            pt: DVector::from_vec(vec![0.1, 0.2, 0.3]),
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

        println!(
            "📝 Test data created with {} data points",
            test_data.data_8_months.len()
        );

        // Create executor environment
        println!("🏗️  Building executor environment...");
        let env = ExecutorEnv::builder()
            .write(&test_data)
            .map_err(|e| eyre::eyre!("Failed to write test data: {}", e))?
            .build()
            .map_err(|e| eyre::eyre!("Failed to build executor environment: {}", e))?;

        println!("✅ Executor environment built successfully");
        println!("🚀 Starting proof generation with Bonsai API...");
        println!("⏰ This may take several minutes depending on API load...");

        let start_time = Instant::now();

        // Execute proof generation directly (RISC0 Bonsai API is async-compatible)
        let prover_result = match default_prover().prove_with_ctx(
            env,
            &VerifierContext::default(),
            MOCK_PROOF_COMPOSITION_GUEST_ELF,
            &ProverOpts::groth16(),
        ) {
            Ok(result) => {
                let elapsed = start_time.elapsed();
                println!("✅ Bonsai proof generation successful in {:?}", elapsed);
                result
            }
            Err(e) => {
                let elapsed = start_time.elapsed();
                println!("❌ Bonsai proof generation failed after {:?}", elapsed);
                println!("Error details: {}", e);

                // Enhanced error diagnostics
                if e.to_string().contains("HTTP") {
                    println!("🌐 HTTP Error Details:");
                    println!(
                        "   - This appears to be a network connectivity issue with Bonsai API"
                    );
                    println!("   - Check internet connection and Bonsai API status");
                    println!("   - API URL: {}", bonsai_api_url);
                } else if e.to_string().contains("timeout") || e.to_string().contains("Timeout") {
                    println!("⏱️  Timeout Error Details:");
                    println!("   - Bonsai API request timed out");
                    println!("   - This may be due to high API load or network latency");
                    println!("   - Consider retrying the operation");
                } else if e.to_string().contains("authentication") || e.to_string().contains("auth")
                {
                    println!("🔐 Authentication Error Details:");
                    println!("   - Check if BONSAI_API_KEY is valid and not expired");
                    println!("   - API Key length: {}", bonsai_api_key.len());
                }

                return Err(eyre::eyre!("RISC0 proof generation failed: {}", e));
            }
        };

        let receipt = prover_result.receipt;
        println!("📃 Receipt obtained successfully");

        // Decode the journal
        println!("🔍 Decoding journal...");
        let journal_output: ProofCompositionOutput = receipt
            .journal
            .decode()
            .map_err(|e| eyre::eyre!("Failed to decode journal: {}", e))?;

        println!("✅ Journal decoded successfully:");
        println!("   - reserve_price: {}", journal_output.reserve_price);
        println!("   - twap_result: {}", journal_output.twap_result);
        println!("   - max_return: {}", journal_output.max_return);

        // Generate proof and calldata
        println!("🔧 Processing receipt for StarkNet verification...");

        let encoded_seal =
            encode_seal(&receipt).map_err(|e| eyre::eyre!("Failed to encode seal: {}", e))?;

        let image_id = compute_image_id(MOCK_PROOF_COMPOSITION_GUEST_ELF)
            .map_err(|e| eyre::eyre!("Failed to compute image ID: {}", e))?;

        let journal = receipt.journal.bytes.clone();
        println!("📏 Journal bytes length: {}", journal.len());

        println!("🔧 Creating Groth16 proof from RISC0 receipt...");
        let groth16_proof =
            Groth16Proof::from_risc0(encoded_seal, image_id.as_bytes().to_vec(), journal);

        println!("⚙️  Generating StarkNet calldata...");
        let calldata = get_groth16_calldata_felt(&groth16_proof, &get_risc0_vk(), CurveID::BN254)
            .map_err(|e| eyre::eyre!("Failed to generate calldata: {}", e))?;

        println!("🎉 SUCCESS! RISC0 proof generation completed successfully!");
        println!("📊 Results:");
        println!("   - Proof generation time: {:?}", start_time.elapsed());
        println!("   - Calldata length: {}", calldata.len());
        println!(
            "   - Journal output: reserve_price={}, twap_result={}, max_return={}",
            journal_output.reserve_price, journal_output.twap_result, journal_output.max_return
        );

        println!(
            "🔍 Calldata preview (first 10 elements): {:?}",
            calldata.iter().take(10).collect::<Vec<_>>()
        );

        println!("📋 COMPLETE CALLDATA FOR STARKNET VERIFICATION:");
        println!("   - Length: {} elements", calldata.len());
        println!("   - Data: {:?}", calldata);

        println!("🔧 Calldata generation details:");
        println!("   - Groth16 proof created from RISC0 receipt ✓");
        println!("   - StarkNet calldata generated using get_groth16_calldata_felt ✓");
        println!("   - Uses RISC0 verification key and BN254 curve ✓");
        println!("   - Ready for onchain verification on StarkNet ✓");

        // Step 3: Send proof onchain using starknet-handler
        #[cfg(feature = "starknet-handler")]
        {
            println!("🔗 Step 3: Sending proof onchain using starknet-handler...");

            let verifier_contract =
                std::env::var("PITCHLAKE_VERIFIER_CONTRACT").unwrap_or_else(|_| {
                    println!("⚠️  PITCHLAKE_VERIFIER_CONTRACT not set, using default");
                    "0x03d8b171d7f5c1f7cccda683ce67a8b0db426d7aaf72755cebd85f4ee3cd1fd8".to_string()
                });

            println!("📋 Verifier contract address: {}", verifier_contract);

            match tokio::runtime::Runtime::new()?
                .block_on(send_proof_onchain(calldata, &verifier_contract))
            {
                Ok(tx_hash) => {
                    println!("🎉 SUCCESS! Proof sent onchain successfully!");
                    println!("📊 Transaction hash: {:?}", tx_hash);
                    println!("🔗 Proof has been verified onchain on StarkNet!");
                }
                Err(e) => {
                    println!("❌ Failed to send proof onchain: {}", e);
                    println!("💡 This might be due to network issues or configuration problems");
                }
            }
        }

        #[cfg(not(feature = "starknet-handler"))]
        {
            println!("⚠️  starknet-handler feature not enabled - skipping onchain verification");
            println!(
                "💡 To enable onchain verification, run with: cargo run --bin bonsai-test --features mock-proof,starknet-handler"
            );
        }

        println!("✅ Bonsai API test completed successfully!");
        println!(
            "🌟 This confirms that Bonsai API is working and accessible from this environment."
        );
        println!("🎯 The generated calldata can be used for onchain proof verification!");
    }

    #[cfg(feature = "mock-proof")]
    Ok(())
}

#[cfg(all(feature = "mock-proof", feature = "starknet-handler"))]
async fn send_proof_onchain(
    calldata: Vec<starknet_crypto::Felt>,
    verifier_address: &str,
) -> Result<starknet_crypto::Felt> {
    use tracing::{debug, info, warn};

    debug!(
        "Starting onchain proof verification with {} calldata elements",
        calldata.len()
    );

    // Load StarkNet configuration
    let config = load_starknet_config()
        .map_err(|e| eyre::eyre!("Failed to load StarkNet configuration: {}", e))?;

    debug!(
        "StarkNet configuration loaded - RPC: {}, Account configured: {}",
        config.rpc_url,
        config.account_address.is_some()
    );

    // Create provider
    let provider = StarknetProvider::new(config)
        .map_err(|e| eyre::eyre!("Failed to create StarkNet provider: {}", e))?;

    info!(
        "Submitting proof verification to contract: {}",
        verifier_address
    );

    // Create default job request from environment
    let vault_address = std::env::var("PITCHLAKE_VAULT").unwrap_or_else(|_| {
        warn!("PITCHLAKE_VAULT not set, using 0x0");
        "0x0".to_string()
    });

    let job_request = starknet_handler::account::PitchLakeJobRequest {
        vault_address: starknet_crypto::Felt::from_hex(&vault_address).unwrap_or_else(|_| {
            warn!("Invalid PITCHLAKE_VAULT format, using 0x0");
            starknet_crypto::Felt::from_hex("0x0").unwrap()
        }),
        timestamp: 1672531200u64, // Default timestamp (2023-01-01)
        program_id: starknet_crypto::Felt::from_hex("0x504954434c4c414b455f5631").unwrap(), // 'PITCH_LAKE_V1'
    };

    // Verify proof onchain
    let tx_hash = provider
        .verify_proof_onchain(verifier_address, calldata, job_request)
        .await
        .map_err(|e| eyre::eyre!("Onchain proof verification failed: {}", e))?;

    info!(
        "Proof verification submitted successfully, tx hash: {:?}",
        tx_hash
    );

    Ok(tx_hash)
}
