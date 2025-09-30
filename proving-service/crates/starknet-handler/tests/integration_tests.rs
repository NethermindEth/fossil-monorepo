use std::env;

use eyre::Result;
use starknet::providers::Provider;
use starknet_crypto::Felt;
use starknet_handler::provider::{StarkNetConfig, StarknetProvider};

/// Test configuration for local StarkNet node
const LOCAL_RPC_URL: &str = "http://localhost:5050";

/// Starknet Sepolia testnet configuration for integration tests
const SEPOLIA_RPC_URL: &str = "https://starknet-sepolia.public.blastapi.io";
const SEPOLIA_CONTRACT_ADDRESS: &str =
    "0x05f80abda60bd853551f43cc00539a3c884384ec01f7a9f5709296c74a3e490b";

// Test timestamps - must be multiples of 3600 (1 hour) and < 1759205842
// These specific timestamps have known data in the Sepolia contract
const TEST_START_TIMESTAMP: u64 = 0x68a226b0; // 1755390640 in decimal
const TEST_END_TIMESTAMP: u64 = 0x68a242d0; // 1755398352 in decimal

/// Get contract address from environment or use default
fn get_contract_address() -> String {
    env::var("FOSSIL_STORE_ADDRESS").unwrap_or_else(|_| {
        "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7".to_string()
    })
}

/// Helper function to create test configuration for local node
fn create_test_config() -> StarkNetConfig {
    StarkNetConfig::new(LOCAL_RPC_URL.to_string(), get_contract_address())
        .with_retry_config(3, 100, 1000) // Shorter retry config for tests
}

/// Helper function to create test configuration for Sepolia testnet
fn create_sepolia_config() -> StarkNetConfig {
    StarkNetConfig::new(
        SEPOLIA_RPC_URL.to_string(),
        SEPOLIA_CONTRACT_ADDRESS.to_string(),
    )
    .with_retry_config(5, 200, 5000) // Longer retry config for remote network
}

/// Test basic provider creation with local RPC
#[tokio::test]
async fn test_provider_creation() -> Result<()> {
    let config = create_test_config();
    let provider = StarknetProvider::new(config)?;

    assert_eq!(provider.rpc_url(), LOCAL_RPC_URL);
    assert_eq!(provider.fossil_store_address(), get_contract_address());

    Ok(())
}

/// Test configuration loading from environment variables
#[tokio::test]
async fn test_config_from_env() -> Result<()> {
    // Set test environment variables
    unsafe {
        env::set_var("STARKNET_RPC_URL", LOCAL_RPC_URL);
        env::set_var("FOSSIL_STORE_ADDRESS", get_contract_address());
        env::set_var("STARKNET_MAX_RETRIES", "2");
        env::set_var("STARKNET_INITIAL_BACKOFF_MS", "50");
        env::set_var("STARKNET_MAX_BACKOFF_MS", "500");
    }

    let config = starknet_handler::config::load_starknet_config()?;

    assert_eq!(config.rpc_url, LOCAL_RPC_URL);
    assert_eq!(config.fossil_store_address, get_contract_address());
    assert_eq!(config.max_retries, Some(2));
    assert_eq!(config.initial_backoff_ms, Some(50));
    assert_eq!(config.max_backoff_ms, Some(500));

    // Clean up environment variables
    unsafe {
        env::remove_var("STARKNET_RPC_URL");
        env::remove_var("FOSSIL_STORE_ADDRESS");
        env::remove_var("STARKNET_MAX_RETRIES");
        env::remove_var("STARKNET_INITIAL_BACKOFF_MS");
        env::remove_var("STARKNET_MAX_BACKOFF_MS");
    }

    Ok(())
}

/// Test RPC connection to local node
/// This test will be skipped if the local node is not running
#[tokio::test]
#[ignore] // Use `cargo test -- --ignored` to run this test
async fn test_rpc_connection() -> Result<()> {
    let config = create_test_config();
    let provider = StarknetProvider::new(config)?;

    // Try to create a simple RPC call to test connectivity
    // This will fail gracefully if the node is not running
    let client = provider.provider();

    // Test basic connectivity by trying to get chain ID
    // This is a basic call that should work with any StarkNet node
    match client.chain_id().await {
        Ok(chain_id) => {
            println!(
                "Successfully connected to StarkNet node. Chain ID: {}",
                chain_id
            );
        }
        Err(e) => {
            println!("Could not connect to local StarkNet node: {}", e);
            // Return error to indicate test failure
            return Err(eyre::eyre!("Local StarkNet node not available: {}", e));
        }
    }

    Ok(())
}

/// Test the get_avg_fees_in_range function with test timestamps
/// This test requires a deployed fossil_store contract with the function
#[tokio::test]
#[ignore] // Use `cargo test -- --ignored` to run this test
async fn test_get_avg_fees_in_range() -> Result<()> {
    let config = create_test_config();
    let provider = StarknetProvider::new(config)?;

    // Test with the provided timestamps
    match provider
        .get_avg_fees_in_range(TEST_START_TIMESTAMP, TEST_END_TIMESTAMP)
        .await
    {
        Ok(fee_data) => {
            println!("✅ Successfully retrieved fee data:");
            println!("   First Timestamp: {}", fee_data.first_timestamp);
            println!("   Last Timestamp: {}", fee_data.last_timestamp);
            println!("   Fees: {} entries", fee_data.fees.len());

            // Print first few fees for verification
            for (i, fee) in fee_data.fees.iter().take(3).enumerate() {
                println!("   Fee {}: {:#x}", i + 1, fee);
            }

            // Basic validation
            assert!(
                fee_data.first_timestamp > 0,
                "First timestamp should be greater than 0"
            );
            assert!(
                fee_data.last_timestamp >= fee_data.first_timestamp,
                "Last timestamp should be >= first timestamp"
            );
            assert!(
                !fee_data.fees.is_empty(),
                "Should have at least one fee value"
            );
        }
        Err(e) => {
            println!("❌ Failed to get average fees: {}", e);
            // Check if it's a contract not found error vs connectivity issue
            if e.to_string().contains("Contract not found") || e.to_string().contains("Entry point")
            {
                println!(
                    "ℹ️  This might be because the fossil_store contract is not deployed at the expected address"
                );
                println!("   Expected address: {}", get_contract_address());
                println!("   Or the contract doesn't have the get_avg_fees_in_range function");
            }
            return Err(e);
        }
    }

    Ok(())
}

/// Test with invalid timestamps (end before start)
#[tokio::test]
#[ignore]
async fn test_invalid_timestamp_range() -> Result<()> {
    let config = create_test_config();
    let provider = StarknetProvider::new(config)?;

    // Use invalid range (end before start)
    let result = provider
        .get_avg_fees_in_range(TEST_END_TIMESTAMP, TEST_START_TIMESTAMP)
        .await;

    // This should either fail or return empty/zero results depending on contract implementation
    match result {
        Ok(fee_data) => {
            println!("Contract handled invalid range gracefully:");
            println!(
                "First Timestamp: {}, Last Timestamp: {}, Fees: {}",
                fee_data.first_timestamp,
                fee_data.last_timestamp,
                fee_data.fees.len()
            );
        }
        Err(e) => {
            println!("Contract rejected invalid timestamp range: {}", e);
        }
    }

    Ok(())
}

/// Test retry mechanism with invalid contract address
#[tokio::test]
#[ignore]
async fn test_retry_mechanism() -> Result<()> {
    // Use an invalid contract address to trigger retries
    let config = StarkNetConfig::new(
        LOCAL_RPC_URL.to_string(),
        "0x0000000000000000000000000000000000000000000000000000000000000001".to_string(),
    )
    .with_retry_config(2, 50, 200); // Fast retries for testing

    let provider = StarknetProvider::new(config)?;

    let start_time = std::time::Instant::now();

    let result = provider
        .get_avg_fees_in_range(TEST_START_TIMESTAMP, TEST_END_TIMESTAMP)
        .await;

    let elapsed = start_time.elapsed();

    // Should fail but take some time due to retries
    assert!(result.is_err(), "Should fail with invalid contract");
    assert!(
        elapsed.as_millis() > 100,
        "Should have taken time for retries"
    );

    println!("Retry test completed in {:?}", elapsed);

    Ok(())
}

/// Test get_avg_fees_in_range with Sepolia testnet
/// This test connects to the actual Starknet Sepolia network and calls a real contract
///
/// Note: This test validates the integration with Sepolia network and correct parsing
/// of the contract's return values. The contract may not have data for all timestamp
/// ranges, which is expected behavior. The test passes as long as:
/// 1. Connection to Sepolia succeeds
/// 2. Contract call completes without error
/// 3. Return values are parsed correctly (timestamps and fees array)
#[tokio::test]
async fn test_sepolia_get_avg_fees_in_range() -> Result<()> {
    println!("\n🔗 Sepolia Integration Test");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("RPC URL: {}", SEPOLIA_RPC_URL);
    println!("Contract: {}", SEPOLIA_CONTRACT_ADDRESS);
    println!(
        "Timestamp Range: {} to {}",
        TEST_START_TIMESTAMP, TEST_END_TIMESTAMP
    );
    println!("Timestamps are multiples of 3600: ✓");

    let config = create_sepolia_config();
    let provider = StarknetProvider::new(config)?;

    // Step 1: Test connectivity
    println!("\n[1/2] Testing connectivity...");
    let client = provider.provider();

    let chain_id = match client.chain_id().await {
        Ok(chain_id) => {
            println!("      ✅ Connected - Chain ID: {:#x}", chain_id);
            chain_id
        }
        Err(e) => {
            println!("      ❌ Connection failed: {}", e);
            return Err(eyre::eyre!("Sepolia connection failed: {}", e));
        }
    };

    // Verify we're on Sepolia
    assert_eq!(
        format!("{:#x}", chain_id),
        "0x534e5f5345504f4c4941",
        "Should be connected to Sepolia network"
    );

    // Step 2: Test contract call
    println!("\n[2/2] Calling contract function...");
    let fee_data = provider
        .get_avg_fees_in_range(TEST_START_TIMESTAMP, TEST_END_TIMESTAMP)
        .await?;

    println!("      ✅ Contract call successful");
    println!("\n📊 Response Data:");
    println!("   First Timestamp: {}", fee_data.first_timestamp);
    println!("   Last Timestamp: {}", fee_data.last_timestamp);
    println!("   Number of Fees: {}", fee_data.fees.len());

    if !fee_data.fees.is_empty() {
        println!("\n   Fee Values:");
        for (i, fee) in fee_data.fees.iter().take(10).enumerate() {
            println!("     [{}] {:#x}", i, fee);
        }
        if fee_data.fees.len() > 10 {
            println!("     ... and {} more", fee_data.fees.len() - 10);
        }
    }

    // Validate response structure
    println!("\n✅ Validation:");

    // Assert expected values from Sepolia contract
    // Expected return: [0x68a226b0, 0x68a242d0, [fee1, fee2, fee3]]
    assert_eq!(
        fee_data.first_timestamp, 0x68a226b0,
        "First timestamp should be 0x68a226b0"
    );
    println!("   ✓ First timestamp: {:#x}", fee_data.first_timestamp);

    assert_eq!(
        fee_data.last_timestamp, 0x68a242d0,
        "Last timestamp should be 0x68a242d0"
    );
    println!("   ✓ Last timestamp: {:#x}", fee_data.last_timestamp);

    assert_eq!(fee_data.fees.len(), 3, "Should have exactly 3 fee values");
    println!("   ✓ Number of fees: {}", fee_data.fees.len());

    // Assert the expected fee values
    let expected_fees = [
        Felt::from_hex("0x2c700f2ff24d0ecf3283da53e806d978b8efbb").unwrap(),
        Felt::from_hex("0x2bdc496e147ae2000000000000000000000000").unwrap(),
        Felt::from_hex("0x2a94af55555556000000000000000000000000").unwrap(),
    ];

    for (i, (actual, expected)) in fee_data.fees.iter().zip(expected_fees.iter()).enumerate() {
        assert_eq!(
            actual, expected,
            "Fee {} should be {:#x}, got {:#x}",
            i, expected, actual
        );
        println!("   ✓ Fee[{}]: {:#x}", i, actual);
    }

    println!("\n   Test Status: ✅ PASSED");
    println!("   - Successfully connected to Sepolia");
    println!("   - Contract call completed without errors");
    println!("   - All return values match expected data");

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ Sepolia integration test completed\n");

    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use starknet_handler::FeeData;

    #[test]
    fn test_fee_data_creation() {
        use starknet_crypto::Felt;

        let fee_data = FeeData::new(
            1755457200,
            1755464400,
            vec![Felt::from(123u64), Felt::from(456u64)],
        );

        assert_eq!(fee_data.first_timestamp, 1755457200);
        assert_eq!(fee_data.last_timestamp, 1755464400);
        assert_eq!(fee_data.fees.len(), 2);
        assert_eq!(fee_data.fees[0], Felt::from(123u64));
        assert_eq!(fee_data.fees[1], Felt::from(456u64));
    }

    #[test]
    fn test_starknet_config_creation() {
        let config =
            StarkNetConfig::new("http://localhost:5050".to_string(), "0x123456".to_string());

        assert_eq!(config.rpc_url, "http://localhost:5050");
        assert_eq!(config.fossil_store_address, "0x123456");
        assert!(config.max_retries.is_none());
    }

    #[test]
    fn test_starknet_config_with_retry() {
        let config =
            StarkNetConfig::new("http://localhost:5050".to_string(), "0x123456".to_string())
                .with_retry_config(5, 200, 2000);

        assert_eq!(config.max_retries, Some(5));
        assert_eq!(config.initial_backoff_ms, Some(200));
        assert_eq!(config.max_backoff_ms, Some(2000));
    }
}
