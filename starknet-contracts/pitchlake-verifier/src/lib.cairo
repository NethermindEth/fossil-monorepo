pub mod groth16_verifier;
//pub use groth16_verifier::Risc0Groth16VerifierBN254;
mod groth16_verifier_constants;
pub mod universal_ecip;
use core::num::traits::{Bounded, WideMul};
use fp::UFixedPoint123x128StorePacking;
pub use universal_ecip::UniversalECIP;
pub mod fixtures;
pub mod pitchlake_verifier;
pub use pitchlake_verifier::PitchLakeVerifier;
pub mod mocks {
    pub mod pitchlake_client;
}
pub use mocks::pitchlake_client::MockPitchLakeClient;

// Constants for byte sizes and offsets
const U64_SIZE: usize = 8;
const U32_SIZE: usize = 4;
const HEX_PREFIX_SIZE: usize = 2; // "0x"
const HEX_HASH_SIZE: usize = 64; // 32 bytes as hex
const HEX_HASH_WITH_PREFIX_SIZE: usize = 66; // "0x" + 64 hex chars
const ASCII_0: u256 = 48;
const ASCII_A_OFFSET: u256 = 87; // 'a' - 10 = 97 - 10 = 87

#[derive(Copy, Debug, Drop, Serde)]
pub struct PitchLakeJobRequest {
    pub vault_address: starknet::ContractAddress, // Which vault is this request for
    // The timestamp the results are for
    pub timestamp: u64,
    // 'PITCH_LAKE_V1' (or program hash when proving ?)
    pub program_id: felt252 // 'PITCH_LAKE_V1'}
}

#[derive(Drop, Debug, Copy, PartialEq, Serde)]
pub struct Journal {
    pub data_8_months_hash: [u32; 8],
    pub start_timestamp: u64,
    pub end_timestamp: u64,
    pub reserve_price: felt252,
    pub floating_point_tolerance: felt252,
    pub reserve_price_tolerance: felt252,
    pub twap_tolerance: felt252,
    pub gradient_tolerance: felt252,
    pub twap_result: felt252,
    pub max_return: felt252,
}

#[derive(Drop, Debug, Copy, PartialEq, Serde)]
pub struct AvgFees {
    pub timestamp: u64,
    pub data_points: u64,
    pub avg_fee: felt252,
}

pub fn decode_journal(journal_bytes: Span<u8>) -> Journal {
    // Parse data_8_months_hash (32 bytes total, as 8 u32 values)
    // First u32 (bytes 0-3)
    let val0: u32 = (*journal_bytes.at(0)).into()
        + (BitShift::shl((*journal_bytes.at(1)).into(), 8))
        + (BitShift::shl((*journal_bytes.at(2)).into(), 16))
        + (BitShift::shl((*journal_bytes.at(3)).into(), 24));

    // Second u32 (bytes 4-7)
    let val1: u32 = (*journal_bytes.at(4)).into()
        + (BitShift::shl((*journal_bytes.at(5)).into(), 8))
        + (BitShift::shl((*journal_bytes.at(6)).into(), 16))
        + (BitShift::shl((*journal_bytes.at(7)).into(), 24));

    // Third u32 (bytes 8-11)
    let val2: u32 = (*journal_bytes.at(8)).into()
        + (BitShift::shl((*journal_bytes.at(9)).into(), 8))
        + (BitShift::shl((*journal_bytes.at(10)).into(), 16))
        + (BitShift::shl((*journal_bytes.at(11)).into(), 24));

    // Fourth u32 (bytes 12-15)
    let val3: u32 = (*journal_bytes.at(12)).into()
        + (BitShift::shl((*journal_bytes.at(13)).into(), 8))
        + (BitShift::shl((*journal_bytes.at(14)).into(), 16))
        + (BitShift::shl((*journal_bytes.at(15)).into(), 24));

    // Fifth u32 (bytes 16-19)
    let val4: u32 = (*journal_bytes.at(16)).into()
        + (BitShift::shl((*journal_bytes.at(17)).into(), 8))
        + (BitShift::shl((*journal_bytes.at(18)).into(), 16))
        + (BitShift::shl((*journal_bytes.at(19)).into(), 24));

    // Sixth u32 (bytes 20-23)
    let val5: u32 = (*journal_bytes.at(20)).into()
        + (BitShift::shl((*journal_bytes.at(21)).into(), 8))
        + (BitShift::shl((*journal_bytes.at(22)).into(), 16))
        + (BitShift::shl((*journal_bytes.at(23)).into(), 24));

    // Seventh u32 (bytes 24-27)
    let val6: u32 = (*journal_bytes.at(24)).into()
        + (BitShift::shl((*journal_bytes.at(25)).into(), 8))
        + (BitShift::shl((*journal_bytes.at(26)).into(), 16))
        + (BitShift::shl((*journal_bytes.at(27)).into(), 24));

    // Eighth u32 (bytes 28-31)
    let val7: u32 = (*journal_bytes.at(28)).into()
        + (BitShift::shl((*journal_bytes.at(29)).into(), 8))
        + (BitShift::shl((*journal_bytes.at(30)).into(), 16))
        + (BitShift::shl((*journal_bytes.at(31)).into(), 24));

    // Create array with the extracted values
    let data_8_months_hash = [val0, val1, val2, val3, val4, val5, val6, val7];

    // Parse start_timestamp (8 bytes)
    let mut byte_offset = 32; // After data_8_months_hash
    let mut start_timestamp: u64 = 0;
    let mut byte_idx = 0;
    while byte_idx < U64_SIZE {
        let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
        let shifted_byte: u64 = BitShift::shl(current_byte, (8 * byte_idx).into());
        start_timestamp += shifted_byte;
        byte_idx += 1;
    }

    // Parse end_timestamp (8 bytes)
    byte_offset += U64_SIZE;
    let mut end_timestamp: u64 = 0;
    let mut byte_idx = 0;
    while byte_idx < U64_SIZE {
        let current_byte: u64 = (*journal_bytes.at(byte_offset + byte_idx)).into();
        let shifted_byte: u64 = BitShift::shl(current_byte, (8 * byte_idx).into());
        end_timestamp += shifted_byte;
        byte_idx += 1;
    }

    // Parse all floating point values
    byte_offset += U64_SIZE;

    // Parse reserve_price (8 bytes)
    let (reserve_price, byte_offset) = parse_packed_fixed_point(journal_bytes, byte_offset);

    // Parse floating_point_tolerance (8 bytes)
    let (floating_point_tolerance, byte_offset) = parse_packed_fixed_point(
        journal_bytes, byte_offset,
    );

    // Parse reserve_price_tolerance (8 bytes)
    let (reserve_price_tolerance, byte_offset) = parse_packed_fixed_point(
        journal_bytes, byte_offset,
    );

    // Parse twap_tolerance (8 bytes)
    let (twap_tolerance, byte_offset) = parse_packed_fixed_point(journal_bytes, byte_offset);

    // Parse gradient_tolerance (8 bytes)
    let (gradient_tolerance, byte_offset) = parse_packed_fixed_point(journal_bytes, byte_offset);

    // Parse twap_result (8 bytes)
    let (twap_result, byte_offset) = parse_packed_fixed_point(journal_bytes, byte_offset);

    // Parse max_return (8 bytes)
    let (max_return, _) = parse_packed_fixed_point(journal_bytes, byte_offset);

    Journal {
        data_8_months_hash,
        start_timestamp,
        end_timestamp,
        reserve_price,
        floating_point_tolerance,
        reserve_price_tolerance,
        twap_tolerance,
        gradient_tolerance,
        twap_result,
        max_return,
    }
}

// Helper function to parse 8 bytes into a UFixedPoint123x128 value
fn parse_packed_fixed_point(journal_bytes: Span<u8>, mut byte_offset: usize) -> (felt252, usize) {
    byte_offset += U32_SIZE; // Skip length indicator (66, 0, 0, 0)
    byte_offset += HEX_PREFIX_SIZE; // Skip "0x" prefix
    let mut value: u256 = 0;
    let mut hex_idx = byte_offset;
    let hex_end = byte_offset + HEX_HASH_SIZE;
    loop {
        if hex_idx >= hex_end {
            break;
        }

        let shifted_hash: u256 = BitShift::shl(value, 4);
        let hex_byte: u256 = (*journal_bytes.at(hex_idx)).into();
        let hex_base: u256 = if hex_byte < 58 { // '0'-'9' vs 'a'-'f'
            ASCII_0 // ASCII '0'
        } else {
            ASCII_A_OFFSET // ASCII 'a' - 10
        };
        value = shifted_hash + hex_byte - hex_base;
        hex_idx += 1;
    }
    byte_offset += HEX_HASH_WITH_PREFIX_SIZE;

    let felt_value: felt252 = value.try_into().unwrap();
    (felt_value, byte_offset)
}

trait BitShift<T> {
    fn shl(x: T, n: T) -> T;
    fn shr(x: T, n: T) -> T;
}

impl U256BitShift of BitShift<u256> {
    fn shl(x: u256, n: u256) -> u256 {
        let res = WideMul::wide_mul(x, pow(2, n));
        u256 { low: res.limb0, high: res.limb1 }
    }

    fn shr(x: u256, n: u256) -> u256 {
        x / pow(2, n)
    }
}

impl U32BitShift of BitShift<u32> {
    fn shl(x: u32, n: u32) -> u32 {
        (WideMul::wide_mul(x, pow(2, n)) & Bounded::<u32>::MAX.into()).try_into().unwrap()
    }

    fn shr(x: u32, n: u32) -> u32 {
        x / pow(2, n)
    }
}

impl U64BitShift of BitShift<u64> {
    fn shl(x: u64, n: u64) -> u64 {
        (WideMul::wide_mul(x, pow(2, n)) & Bounded::<u64>::MAX.into()).try_into().unwrap()
    }

    fn shr(x: u64, n: u64) -> u64 {
        x / pow(2, n)
    }
}

impl U128BitShift of BitShift<u128> {
    fn shl(x: u128, n: u128) -> u128 {
        let res = WideMul::wide_mul(x, pow(2, n));
        res.low
    }

    fn shr(x: u128, n: u128) -> u128 {
        x / pow(2, n)
    }
}

fn pow<T, +Sub<T>, +Mul<T>, +Div<T>, +Rem<T>, +PartialEq<T>, +Into<u8, T>, +Drop<T>, +Copy<T>>(
    base: T, exp: T,
) -> T {
    if exp == 0_u8.into() {
        1_u8.into()
    } else if exp == 1_u8.into() {
        base
    } else if exp % 2_u8.into() == 0_u8.into() {
        pow(base * base, exp / 2_u8.into())
    } else {
        base * pow(base * base, exp / 2_u8.into())
    }
}

#[cfg(test)]
mod tests {
    use fp::{UFixedPoint123x128StorePacking as SP, UFixedPointTrait};
    use super::*;

    #[derive(Drop, Debug, Copy, PartialEq, Serde)]
    pub struct TestJournal {
        pub data_8_months_hash: [u32; 8],
        pub start_timestamp: u64,
        pub end_timestamp: u64,
        pub reserve_price: u256,
        pub floating_point_tolerance: u256,
        pub reserve_price_tolerance: u256,
        pub twap_tolerance: u256,
        pub gradient_tolerance: u256,
        pub twap_result: u256,
        pub max_return: u256,
    }

    #[test]
    fn decode_journal_test() {
        let journal_bytes = get_journal_bytes();
        let journal = decode_journal(journal_bytes);
        let expected_journal = get_expected_results();
        // Verify data_8_months_hash (converting to the expected array of u32)

        assert_eq!(journal.data_8_months_hash, expected_journal.data_8_months_hash);

        // Verify timestamps
        assert_eq!(journal.start_timestamp, expected_journal.start_timestamp);
        assert_eq!(journal.end_timestamp, expected_journal.end_timestamp);

        // For the floating point values, we would need to check the bit representation
        // or approximate values, depending on how UFixedPoint123x128 stores values
        assert_eq!(
            SP::unpack(journal.reserve_price).get_integer(), expected_journal.reserve_price.high,
        );
        assert_eq!(
            SP::unpack(journal.reserve_price).get_fractional(), expected_journal.reserve_price.low,
        );
        assert_eq!(
            SP::unpack(journal.floating_point_tolerance).get_integer(),
            expected_journal.floating_point_tolerance.high,
        );
        assert_eq!(
            SP::unpack(journal.floating_point_tolerance).get_fractional(),
            expected_journal.floating_point_tolerance.low,
        );
        assert_eq!(
            SP::unpack(journal.reserve_price_tolerance).get_integer(),
            expected_journal.reserve_price_tolerance.high,
        );
        assert_eq!(
            SP::unpack(journal.reserve_price_tolerance).get_fractional(),
            expected_journal.reserve_price_tolerance.low,
        );
        assert_eq!(
            SP::unpack(journal.twap_tolerance).get_integer(), expected_journal.twap_tolerance.high,
        );
        assert_eq!(
            SP::unpack(journal.twap_tolerance).get_fractional(),
            expected_journal.twap_tolerance.low,
        );
        assert_eq!(
            SP::unpack(journal.gradient_tolerance).get_integer(),
            expected_journal.gradient_tolerance.high,
        );
        assert_eq!(
            SP::unpack(journal.gradient_tolerance).get_fractional(),
            expected_journal.gradient_tolerance.low,
        );
        assert_eq!(
            SP::unpack(journal.twap_result).get_integer(), expected_journal.twap_result.high,
        );
        assert_eq!(
            UFixedPoint123x128StorePacking::unpack(journal.twap_result).get_fractional(),
            expected_journal.twap_result.low,
        );
        assert_eq!(
            UFixedPoint123x128StorePacking::unpack(journal.max_return).get_integer(),
            expected_journal.max_return.high,
        );
        assert_eq!(
            UFixedPoint123x128StorePacking::unpack(journal.max_return).get_fractional(),
            expected_journal.max_return.low,
        );
    }

    fn get_expected_results() -> TestJournal {
        TestJournal {
            data_8_months_hash: [
                176682157, 3315611904, 69122759, 3259044264, 1698705339, 1448440140, 3846648702,
                370555961,
            ],
            start_timestamp: 1708833600,
            end_timestamp: 1716609600,
            reserve_price: u256 { high: 2436485959, low: 159863518606830028081101360966223790080 },
            floating_point_tolerance: u256 { high: 0, low: 3402823669209384912995114146594816 },
            reserve_price_tolerance: u256 { high: 5, low: 0 },
            twap_tolerance: u256 { high: 1, low: 0 },
            gradient_tolerance: u256 { high: 0, low: 17014118346046924117642026945517453312 },
            twap_result: u256 { high: 14346521680, low: 192471954174812891655089835803777433600 },
            max_return: u256 { high: 1, low: 183365823839893747160194852195351396352 },
        }
    }

    fn get_journal_bytes() -> Span<u8> {
        array![
            173, 244, 135, 10, 0, 57, 160, 197, 199, 186, 30, 4, 168, 17, 65, 194, 187, 47, 64, 101,
            76, 113, 85, 86, 126, 51, 71, 229, 57, 60, 22, 22, 64, 187, 218, 101, 0, 0, 0, 0, 64,
            98, 81, 102, 0, 0, 0, 0, 66, 0, 0, 0, 48, 120, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 57, 49, 51, 57, 100, 51, 52, 55,
            55, 56, 52, 52, 57, 56, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 0, 0, 66, 0, 0, 0, 48, 120, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 97, 55, 99, 53, 97, 99, 52, 55, 49, 98, 52, 55, 56,
            56, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 0, 0, 66, 0, 0, 0, 48, 120,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 53, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 0, 0,
            66, 0, 0, 0, 48, 120, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 49, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 0, 0, 66, 0, 0, 0, 48, 120, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 100, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 0, 0, 66, 0, 0, 0, 48, 120, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 51, 53, 55, 49, 101,
            56, 99, 53, 48, 57, 48, 99, 99, 99, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 0, 0, 66, 0, 0, 0, 48, 120, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 49, 56, 57, 102, 50, 102, 57, 49, 99, 55, 101, 98, 53,
            101, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 0, 0,
        ]
            .span()
    }
}
