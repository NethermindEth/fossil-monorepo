use eyre::Result;
use starknet_crypto::Felt;
use std::sync::OnceLock;

/// Mock data generator for testing purposes
/// Generates realistic fee data that matches the expected format from onchain sources
pub struct MockFeeDataGenerator {
    base_fee: u128,
    variance: u128,
}

impl MockFeeDataGenerator {
    /// Creates a new mock data generator with a base fee value and variance
    pub fn new(base_fee: u128, variance: u128) -> Self {
        Self { base_fee, variance }
    }

    /// Creates a default generator with realistic fee values
    pub fn default() -> Self {
        // Base fee similar to the provided example: 954945341986763873307577006398851026606748675
        // Since u128 max is ~3.4e38, we need to use a smaller base value and scale appropriately
        Self::new(95494534198676387330u128, 5000000000000000000u128)
    }

    /// Generates exactly 5760 mock fee values (8 months of hourly data)
    /// Returns vector of hex-encoded fee values as strings
    pub fn generate_mock_fee_data(&self) -> Vec<String> {
        let mut fee_data = Vec::with_capacity(5760);

        // Use a simple deterministic approach for reproducible testing
        for i in 0..5760 {
            let fee_value = self.calculate_fee_for_hour(i);
            let hex_string = format!("{:#x}", fee_value);
            fee_data.push(hex_string);
        }

        fee_data
    }

    /// Generates exactly 5760 mock fee values as Felt objects for RISC0 processing
    pub fn generate_mock_fee_data_as_felts(&self) -> Result<Vec<Felt>> {
        let mut fee_data = Vec::with_capacity(5760);

        for i in 0..5760 {
            let fee_value = self.calculate_fee_for_hour(i);
            let felt = Felt::from(fee_value);
            fee_data.push(felt);
        }

        Ok(fee_data)
    }

    /// Calculate a fee value for a given hour with some variation
    /// Uses a combination of base fee + sine wave variation + hourly increment
    fn calculate_fee_for_hour(&self, hour: usize) -> u128 {
        // Add some cyclical variation (simulating daily/weekly patterns)
        let daily_cycle = ((hour as f64 % 24.0) * std::f64::consts::PI / 12.0).sin();
        let weekly_cycle = ((hour as f64 % 168.0) * std::f64::consts::PI / 84.0).sin();

        // Combine variations
        let variation_factor = (daily_cycle + weekly_cycle) / 2.0;
        let variation = (self.variance as f64 * variation_factor) as i128;

        // Add a small linear trend over time
        let trend = (hour as u128) * 100000000000000000u128;

        // Calculate final fee (ensure it stays positive)
        let base_with_trend = self.base_fee + trend;
        if variation >= 0 {
            base_with_trend + (variation as u128)
        } else {
            let abs_variation = (-variation) as u128;
            if abs_variation < base_with_trend {
                base_with_trend - abs_variation
            } else {
                base_with_trend / 2 // Fallback to prevent underflow
            }
        }
    }
}

/// Global instance for consistent mock data across the application
static MOCK_GENERATOR: OnceLock<MockFeeDataGenerator> = OnceLock::new();

/// Get the global mock data generator instance
pub fn get_mock_generator() -> &'static MockFeeDataGenerator {
    MOCK_GENERATOR.get_or_init(|| MockFeeDataGenerator::default())
}

/// Generate mock verification hash for testing
/// Returns a consistent but realistic-looking hash for testing purposes
pub fn generate_mock_verification_hash(start_timestamp: u64) -> [u32; 8] {
    // Generate a deterministic but varied hash based on timestamp
    let mut hash = [0u32; 8];
    let base = start_timestamp as u32;

    for i in 0..8 {
        hash[i] = base
            .wrapping_add(i as u32 * 0x12345678)
            .wrapping_mul(0x9abcdef0);
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_generator_creates_correct_amount_of_data() {
        let generator = MockFeeDataGenerator::default();
        let fee_data = generator.generate_mock_fee_data();

        assert_eq!(
            fee_data.len(),
            5760,
            "Should generate exactly 5760 fee values"
        );
    }

    #[test]
    fn test_mock_generator_creates_valid_hex_strings() {
        let generator = MockFeeDataGenerator::default();
        let fee_data = generator.generate_mock_fee_data();

        for (i, hex_str) in fee_data.iter().take(10).enumerate() {
            assert!(
                hex_str.starts_with("0x"),
                "Fee {} should start with 0x: {}",
                i,
                hex_str
            );

            // Verify it can be parsed as a valid hex number
            let without_prefix = hex_str.strip_prefix("0x").unwrap();
            assert!(
                u128::from_str_radix(without_prefix, 16).is_ok(),
                "Fee {} should be valid hex: {}",
                i,
                hex_str
            );
        }
    }

    #[test]
    fn test_mock_generator_creates_felts() {
        let generator = MockFeeDataGenerator::default();
        let fee_data = generator.generate_mock_fee_data_as_felts().unwrap();

        assert_eq!(
            fee_data.len(),
            5760,
            "Should generate exactly 5760 Felt values"
        );

        // Verify all values are valid Felt objects
        for (i, felt) in fee_data.iter().take(10).enumerate() {
            // Just verify we can convert back to string (basic validity check)
            let hex_str = format!("{:#x}", felt);
            assert!(
                hex_str.starts_with("0x"),
                "Felt {} should convert to valid hex: {}",
                i,
                hex_str
            );
        }
    }

    #[test]
    fn test_mock_generator_deterministic() {
        let generator = MockFeeDataGenerator::default();
        let fee_data_1 = generator.generate_mock_fee_data();
        let fee_data_2 = generator.generate_mock_fee_data();

        assert_eq!(
            fee_data_1, fee_data_2,
            "Mock data generation should be deterministic"
        );
    }

    #[test]
    fn test_mock_verification_hash() {
        let hash1 = generate_mock_verification_hash(1234567890);
        let hash2 = generate_mock_verification_hash(1234567890);
        let hash3 = generate_mock_verification_hash(1234567891);

        assert_eq!(hash1, hash2, "Same timestamp should produce same hash");
        assert_ne!(
            hash1, hash3,
            "Different timestamps should produce different hashes"
        );
        assert_eq!(hash1.len(), 8, "Hash should have 8 u32 elements");
    }

    #[test]
    fn test_fee_values_in_reasonable_range() {
        let generator = MockFeeDataGenerator::default();
        let fee_data = generator.generate_mock_fee_data();

        for (i, hex_str) in fee_data.iter().take(100).enumerate() {
            let without_prefix = hex_str.strip_prefix("0x").unwrap();
            let value = u128::from_str_radix(without_prefix, 16).unwrap();

            // Ensure values are in a reasonable range (not zero, not absurdly large)
            assert!(value > 0, "Fee {} should be positive: {}", i, value);
            // Should be roughly in the ballpark of the scaled example value
            assert!(
                value > 80000000000000000000u128,
                "Fee {} seems too small: {}",
                i,
                value
            );
            assert!(
                value < 120000000000000000000u128,
                "Fee {} seems too large: {}",
                i,
                value
            );
        }
    }
}
