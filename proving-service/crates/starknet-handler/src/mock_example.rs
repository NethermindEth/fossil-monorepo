/// Mock data example functionality for integration testing
/// This demonstrates how the mock data system works

#[cfg(test)]
mod tests {
    use crate::{config::load_starknet_config, provider::StarknetProvider};
    use std::env;

    #[tokio::test]
    #[serial_test::serial]
    async fn test_mock_data_integration() {
        // Set mock data environment variable
        unsafe {
            env::set_var("USE_MOCK_STARKNET_DATA", "true");
            env::set_var(
                "FOSSIL_STORE_ADDRESS",
                "0x027a6e498bfad98a5145ac507f90fac6268cce6c0373c6eecd38de46be655b55",
            );
        }

        let config = load_starknet_config().expect("Failed to load config");
        assert!(config.use_mock_data, "Mock data should be enabled");

        let provider = StarknetProvider::new(config).expect("Failed to create provider");

        // Test timestamps
        let end_timestamp = 1700000000u64; // Fixed timestamp for reproducible tests
        let start_timestamp = end_timestamp - (8 * 30 * 24 * 3600); // ~8 months ago

        // Test get_avg_fees_in_range
        let fee_data = provider
            .get_avg_fees_in_range(start_timestamp, end_timestamp)
            .await
            .expect("Should get mock fee data");

        assert_eq!(
            fee_data.block_hashes.len(),
            5760,
            "Should return exactly 5760 mock fee values"
        );
        assert_eq!(
            fee_data.avg_l1_gas_fee, 1_000_000_000u64,
            "Should return mock L1 fee"
        );
        assert_eq!(
            fee_data.avg_l2_gas_fee, 500_000_000u64,
            "Should return mock L2 fee"
        );

        // Verify first few values are valid hex
        for (i, hash) in fee_data.block_hashes.iter().take(10).enumerate() {
            assert!(
                hash.starts_with("0x"),
                "Hash {} should start with 0x: {}",
                i,
                hash
            );
            assert!(
                hash.len() > 2,
                "Hash {} should have content after 0x: {}",
                i,
                hash
            );
        }

        // Test get_raw_fees_in_range
        let raw_fees = provider
            .get_raw_fees_in_range(start_timestamp, end_timestamp)
            .await
            .expect("Should get mock raw fee data");

        assert_eq!(
            raw_fees.len(),
            5760,
            "Should return exactly 5760 mock raw fee values"
        );

        // Test get_verification_hash
        let hash = provider
            .get_verification_hash(start_timestamp)
            .await
            .expect("Should get mock verification hash");

        assert_eq!(hash.len(), 8, "Verification hash should have 8 elements");

        // Test deterministic behavior
        let hash2 = provider
            .get_verification_hash(start_timestamp)
            .await
            .expect("Should get same mock verification hash");

        assert_eq!(hash, hash2, "Verification hash should be deterministic");

        // Clean up
        unsafe {
            env::remove_var("USE_MOCK_STARKNET_DATA");
            env::remove_var("FOSSIL_STORE_ADDRESS");
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_mock_data_disabled_by_default() {
        // Make sure mock data is not set
        unsafe {
            env::remove_var("USE_MOCK_STARKNET_DATA");
            env::set_var(
                "FOSSIL_STORE_ADDRESS",
                "0x027a6e498bfad98a5145ac507f90fac6268cce6c0373c6eecd38de46be655b55",
            );
        }

        let config = load_starknet_config().expect("Failed to load config");
        assert!(
            !config.use_mock_data,
            "Mock data should be disabled by default"
        );

        // Clean up
        unsafe {
            env::remove_var("FOSSIL_STORE_ADDRESS");
        }
    }
}
