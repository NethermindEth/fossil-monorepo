use std::env;

use eyre::Result;
use starknet::providers::Provider;
use starknet_handler::provider::{StarkNetConfig, StarknetProvider};

/// Test configuration for local StarkNet node
const LOCAL_RPC_URL: &str = "http://localhost:5050";
const TEST_START_TIMESTAMP: u64 = 1755457200; // Earlier timestamp
const TEST_END_TIMESTAMP: u64 = 1755464400; // Later timestamp

/// Get contract address from environment or use default
fn get_contract_address() -> String {
    env::var("FOSSIL_STORE_ADDRESS").unwrap_or_else(|_| {
        "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7".to_string()
    })
}

/// Helper function to create test configuration
fn create_test_config() -> StarkNetConfig {
    StarkNetConfig::new(LOCAL_RPC_URL.to_string(), get_contract_address())
        .with_retry_config(3, 100, 1000) // Shorter retry config for tests
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
            println!("   L1 Gas Fee: {}", fee_data.avg_l1_gas_fee);
            println!("   L2 Gas Fee: {}", fee_data.avg_l2_gas_fee);
            println!("   Block Hashes: {} entries", fee_data.block_hashes.len());

            // Print first few hashes for verification
            for (i, hash) in fee_data.block_hashes.iter().take(3).enumerate() {
                println!("   Hash {}: {}", i + 1, hash);
            }

            // Basic validation
            assert!(
                fee_data.avg_l1_gas_fee > 0,
                "L1 gas fee should be greater than 0"
            );
            assert!(
                fee_data.avg_l2_gas_fee > 0,
                "L2 gas fee should be greater than 0"
            );
            assert!(
                !fee_data.block_hashes.is_empty(),
                "Should have at least one block hash"
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
                "L1 Fee: {}, L2 Fee: {}, Hashes: {}",
                fee_data.avg_l1_gas_fee,
                fee_data.avg_l2_gas_fee,
                fee_data.block_hashes.len()
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

#[cfg(test)]
mod unit_tests {
    use super::*;
    use starknet_handler::FeeData;

    #[test]
    fn test_fee_data_creation() {
        let fee_data = FeeData::new(
            1000000,
            500000,
            vec!["0x123".to_string(), "0x456".to_string()],
        );

        assert_eq!(fee_data.avg_l1_gas_fee, 1000000);
        assert_eq!(fee_data.avg_l2_gas_fee, 500000);
        assert_eq!(fee_data.block_hashes.len(), 2);
        assert_eq!(fee_data.block_hashes[0], "0x123");
        assert_eq!(fee_data.block_hashes[1], "0x456");
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
