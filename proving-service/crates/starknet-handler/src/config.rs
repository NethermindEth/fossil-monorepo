use std::env;

use eyre::{Result, eyre};
use tracing::{debug, info};

use crate::provider::StarkNetConfig;

/// Environment variable names for StarkNet configuration
pub const STARKNET_RPC_ENV: &str = "STARKNET_RPC_URL";
pub const FOSSIL_STORE_ADDRESS_ENV: &str = "FOSSIL_STORE_ADDRESS";
pub const HASH_STORE_ADDRESS_ENV: &str = "HASH_STORE_ADDRESS";
pub const STARKNET_ACCOUNT_ADDRESS_ENV: &str = "STARKNET_ACCOUNT_ADDRESS";
pub const STARKNET_ACCOUNT_PRIVATE_KEY_ENV: &str = "STARKNET_PRIVATE_KEY";
pub const STARKNET_MAX_RETRIES_ENV: &str = "STARKNET_MAX_RETRIES";
pub const STARKNET_INITIAL_BACKOFF_MS_ENV: &str = "STARKNET_INITIAL_BACKOFF_MS";
pub const STARKNET_MAX_BACKOFF_MS_ENV: &str = "STARKNET_MAX_BACKOFF_MS";

/// Default values for StarkNet configuration
pub const DEFAULT_STARKNET_RPC: &str = "https://starknet-mainnet.public.blastapi.io";

/// Loads StarkNet configuration from environment variables
pub fn load_starknet_config() -> Result<StarkNetConfig> {
    debug!("Loading StarkNet configuration from environment");

    // Load required configuration
    let rpc_url = env::var(STARKNET_RPC_ENV).unwrap_or_else(|_| {
        info!("Using default StarkNet RPC URL: {}", DEFAULT_STARKNET_RPC);
        DEFAULT_STARKNET_RPC.to_string()
    });

    let fossil_store_address = env::var(FOSSIL_STORE_ADDRESS_ENV)
        .map_err(|_| eyre!("FOSSIL_STORE_ADDRESS environment variable is required"))?;

    let hash_store_address = env::var(HASH_STORE_ADDRESS_ENV).ok();
    let account_address = env::var(STARKNET_ACCOUNT_ADDRESS_ENV).ok();
    let account_private_key = env::var(STARKNET_ACCOUNT_PRIVATE_KEY_ENV).ok();

    info!(
        rpc_url = %rpc_url,
        fossil_store_address = %fossil_store_address,
        hash_store_address = ?hash_store_address,
        account_configured = account_address.is_some() && account_private_key.is_some(),
        "Loaded StarkNet configuration"
    );

    let mut config = StarkNetConfig::new(rpc_url, fossil_store_address);

    // Add hash store address if provided
    if let Some(hash_store_addr) = hash_store_address {
        config = config.with_hash_store(hash_store_addr);
    }

    // Add account credentials if both are provided
    if let (Some(addr), Some(key)) = (account_address, account_private_key) {
        config = config.with_account(addr, key);
        debug!("Account credentials configured for write operations");
    }

    // Load optional retry configuration
    if let (Ok(max_retries), Ok(initial_backoff), Ok(max_backoff)) = (
        env::var(STARKNET_MAX_RETRIES_ENV)
            .and_then(|s| s.parse::<u32>().map_err(|_| env::VarError::NotPresent)),
        env::var(STARKNET_INITIAL_BACKOFF_MS_ENV)
            .and_then(|s| s.parse::<u64>().map_err(|_| env::VarError::NotPresent)),
        env::var(STARKNET_MAX_BACKOFF_MS_ENV)
            .and_then(|s| s.parse::<u64>().map_err(|_| env::VarError::NotPresent)),
    ) {
        config = config.with_retry_config(max_retries, initial_backoff, max_backoff);
        debug!(
            max_retries,
            initial_backoff, max_backoff, "Loaded custom retry configuration"
        );
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    #[serial_test::serial]
    fn test_load_starknet_config_with_required_env() {
        // Set required environment variables
        unsafe {
            env::set_var(FOSSIL_STORE_ADDRESS_ENV, "0x1234567890abcdef");
        }

        let result = load_starknet_config();
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.rpc_url, DEFAULT_STARKNET_RPC);
        assert_eq!(config.fossil_store_address, "0x1234567890abcdef");

        // Clean up
        unsafe {
            env::remove_var(FOSSIL_STORE_ADDRESS_ENV);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_load_starknet_config_with_custom_rpc() {
        // Set environment variables
        unsafe {
            env::set_var(STARKNET_RPC_ENV, "http://localhost:5050");
            env::set_var(FOSSIL_STORE_ADDRESS_ENV, "0x1234567890abcdef");
        }

        let result = load_starknet_config();
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.rpc_url, "http://localhost:5050");
        assert_eq!(config.fossil_store_address, "0x1234567890abcdef");

        // Clean up
        unsafe {
            env::remove_var(STARKNET_RPC_ENV);
            env::remove_var(FOSSIL_STORE_ADDRESS_ENV);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_load_starknet_config_with_retry_config() {
        // Set all environment variables
        unsafe {
            env::set_var(STARKNET_RPC_ENV, "http://localhost:5050");
            env::set_var(FOSSIL_STORE_ADDRESS_ENV, "0x1234567890abcdef");
            env::set_var(STARKNET_MAX_RETRIES_ENV, "3");
            env::set_var(STARKNET_INITIAL_BACKOFF_MS_ENV, "200");
            env::set_var(STARKNET_MAX_BACKOFF_MS_ENV, "5000");
        }

        let result = load_starknet_config();
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.max_retries, Some(3));
        assert_eq!(config.initial_backoff_ms, Some(200));
        assert_eq!(config.max_backoff_ms, Some(5000));

        // Clean up
        unsafe {
            env::remove_var(STARKNET_RPC_ENV);
            env::remove_var(FOSSIL_STORE_ADDRESS_ENV);
            env::remove_var(STARKNET_MAX_RETRIES_ENV);
            env::remove_var(STARKNET_INITIAL_BACKOFF_MS_ENV);
            env::remove_var(STARKNET_MAX_BACKOFF_MS_ENV);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_load_starknet_config_missing_required() {
        // Create a custom test that validates the error case
        // We'll use a different environment variable name to test the pattern

        // Backup and clear the required environment variable
        let backup = env::var(FOSSIL_STORE_ADDRESS_ENV).ok();

        unsafe {
            env::remove_var(FOSSIL_STORE_ADDRESS_ENV);
        }

        // Wait a moment to ensure the variable is properly removed
        std::thread::sleep(std::time::Duration::from_millis(1));

        // Verify it's missing and test should fail
        if env::var(FOSSIL_STORE_ADDRESS_ENV).is_err() {
            let result = load_starknet_config();
            assert!(
                result.is_err(),
                "Should fail when required env var is missing"
            );
        } else {
            // If we can't create the test conditions, skip the test
            println!("Warning: Could not remove environment variable for test");
        }

        // Always restore the environment
        if let Some(value) = backup {
            unsafe {
                env::set_var(FOSSIL_STORE_ADDRESS_ENV, value);
            }
        } else {
            // Set a default test value
            unsafe {
                env::set_var(
                    FOSSIL_STORE_ADDRESS_ENV,
                    "0x027a6e498bfad98a5145ac507f90fac6268cce6c0373c6eecd38de46be655b55",
                );
            }
        }
    }
}
