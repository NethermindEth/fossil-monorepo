#![deny(unused_crate_dependencies)]

#[cfg(feature = "binary")]
use dotenv as _;
#[cfg(feature = "binary")]
use tracing_subscriber as _;

pub mod account;
pub mod config;
pub mod example;
pub mod mock_data;
pub mod mock_example;
pub mod provider;

use eyre::{Result, eyre};
use starknet::core::types::U256;
use starknet_crypto::Felt;
use tracing::{debug, instrument};

/// Represents the average fees and hash data returned from the fossil_store contract
#[derive(Clone, Debug)]
pub struct FeeData {
    pub avg_l1_gas_fee: u64,
    pub avg_l2_gas_fee: u64,
    pub block_hashes: Vec<String>,
}

impl FeeData {
    pub fn new(avg_l1_gas_fee: u64, avg_l2_gas_fee: u64, block_hashes: Vec<String>) -> Self {
        Self {
            avg_l1_gas_fee,
            avg_l2_gas_fee,
            block_hashes,
        }
    }
}

/// Represents fee data with cryptographic verification hash for RISC0 proof generation
#[derive(Clone, Debug)]
pub struct FeeDataWithHash {
    pub raw_fees: Vec<Felt>,
    pub verification_hash: [u32; 8],
    pub avg_l1_gas_fee: u64,
    pub avg_l2_gas_fee: u64,
}

impl FeeDataWithHash {
    pub fn new(
        raw_fees: Vec<Felt>,
        verification_hash: [u32; 8],
        avg_l1_gas_fee: u64,
        avg_l2_gas_fee: u64,
    ) -> Self {
        Self {
            raw_fees,
            verification_hash,
            avg_l1_gas_fee,
            avg_l2_gas_fee,
        }
    }
}

#[instrument(level = "debug")]
pub fn u256_from_hex(hex: &str) -> Result<U256> {
    let hex_clean = hex.strip_prefix("0x").unwrap_or(hex);

    // Validate hex string length (can be up to 64 characters for U256)
    if hex_clean.len() > 64 {
        return Err(eyre!("Invalid hex string length: {}", hex_clean.len()));
    }

    // Validate hex characters
    if !hex_clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(eyre!("Invalid hex characters: {}", hex_clean));
    }

    // Parse the hex string as a big integer and convert to U256
    // Split into high and low parts if needed
    let padded = format!("{:0>64}", hex_clean);

    let high_str = &padded[0..32];
    let low_str = &padded[32..64];

    let high = u128::from_str_radix(high_str, 16)
        .map_err(|e| eyre!("Failed to parse high part: {}", e))?;
    let low =
        u128::from_str_radix(low_str, 16).map_err(|e| eyre!("Failed to parse low part: {}", e))?;

    let result = U256::from_words(low, high);

    debug!(result = ?result, "Hex conversion completed");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
