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

/// Represents the fee data returned from the fossil_store contract
/// The contract returns (first_timestamp, last_timestamp, Array<felt252>)
/// where the Array contains the actual fee values
#[derive(Clone, Debug)]
pub struct FeeData {
    pub first_timestamp: u64,
    pub last_timestamp: u64,
    pub fees: Vec<Felt>,
}

impl FeeData {
    pub fn new(first_timestamp: u64, last_timestamp: u64, fees: Vec<Felt>) -> Self {
        Self {
            first_timestamp,
            last_timestamp,
            fees,
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
            1755457200,
            1755464400,
            vec![Felt::from(123u64), Felt::from(456u64)],
        );

        assert_eq!(fee_data.first_timestamp, 1755457200);
        assert_eq!(fee_data.last_timestamp, 1755464400);
        assert_eq!(fee_data.fees.len(), 2);
    }
}
