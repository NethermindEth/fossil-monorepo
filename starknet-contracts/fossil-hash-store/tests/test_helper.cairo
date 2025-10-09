use core::sha256::compute_sha256_u32_array;
use fp::UFixedPoint123x128StorePacking;
use sha2_input::helper::Helper::*;

#[test]
fn test_convert_avg_fees_to_u32_array_and_hash() {
    let avg_fees: felt252 = 12345678901234567890;
    let res = convert_avg_fees_to_u32_array(avg_fees);
    assert(res == array![3944680146, 2874452364, 0, 0, 0, 0, 0, 0], 'invalid result');

    let hash_res = compute_sha256_u32_array(res.clone(), 0, 0);
    assert(
        hash_res == [
            2891502308, 2985974267, 273846311, 3274118614, 3005251579, 4112087022, 2920814702,
            1397971266,
        ], // ac58d2e4 b1fa59fb 10529027 c32715d6 b3207ffb f51977ee ae18186e 53535942
        'invalid hash result',
    );
}

#[ignore]
#[test]
// total step required for this is 216174982.
// to run this test, we need to apply --max-n-steps flag when we are running the test
fn test_append_5760_avg_fees() {
    let avg_gas_fee_packed: felt252 = 565966358523639806057303238395575262125865566208;

    let mut result_array = array![];
    for _ in 0..5760_u32 {
        // let timestamp_array = convert_timestamp_to_u32_array(timestamp);
        let avg_fees_array = convert_avg_fees_to_u32_array(avg_gas_fee_packed);
        result_array.append_span(avg_fees_array.span());
    }
    let _hash_res = compute_sha256_u32_array(result_array, 0, 0);
}

#[test]
fn test_append_180_avg_fees_and_hash() {
    let avg_gas_fee_packed: felt252 = 565966358523639806057303238395575262125865566208;

    let mut input_array = array![];
    for _ in 0..180_u32 {
        input_array.append(avg_gas_fee_packed);
    }
    let hash_res = hash_of_avg_fees(input_array);
    assert(
        hash_res == [
            0x71316d72, 0x99f3b0a0, 0xf67978f3, 0x2f96f5de, 0x0a358b38, 0x65b4a286, 0x568c4b8a,
            0x833531ef,
        ],
        'invalid hash result',
    );
}

#[test]
fn test_hash_of_hash_of_avg_fees() {
    let hash_of_avg_fees = [
        1899064690_u32, 2582884512, 4135155955, 798422494, 171281208, 1706336902, 1452034954,
        2201301487,
    ];

    let mut input_array = array![];
    for _ in 0..32_u32 {
        input_array.append(hash_of_avg_fees);
    }
    let hash_res = hash_of_hash_of_avg_fees(input_array);
    assert(
        hash_res == [
            0xa375b797, 0x375dca1e, 0x2eb59bf5, 0xe5743db6, 0x343351e6, 0xa16ea306, 0xb40c72b8,
            0xa68e3133,
        ],
        'invalid hash result',
    );
}

#[test]
fn test_hash_batched_avg_fees2() {
    let hash_of_avg_fees = [
        1899064690_u32, 2582884512, 4135155955, 798422494, 171281208, 1706336902, 1452034954,
        2201301487,
    ];
    let mut result_array = array![];
    let num_in_a_batch = 32_u64; // 5760/180
    for _ in 0..num_in_a_batch {
        result_array.append_span(hash_of_avg_fees.span());
    }

    let hash_res = compute_sha256_u32_array(result_array, 0, 0);
    assert(
        hash_res == [
            0xa375b797, 0x375dca1e, 0x2eb59bf5, 0xe5743db6, 0x343351e6, 0xa16ea306, 0xb40c72b8,
            0xa68e3133,
        ],
        'invalid hash result',
    );
}
// this result is crossedcheck against the rust implementation
#[test]
fn test_correct_hash_0() {
    let input = array![1_u32, 2_u32];

    // it seems like each of the element in the u32 array is big endian
    let hash_res = compute_sha256_u32_array(input, 0, 0);
    println!("hash_res: {:?}", hash_res);
    // [257449429, 418186820, 4092460351, 2102399692, 2672057420, 2636439912, 3868509398,
// 4084182694]

    // in rust:
// let inputs = vec![1_u32, 2_u32];

    // let mut input_to_hash = vec![];
// for input in inputs {
//     let input_bytes = input.to_be_bytes();
//     input_to_hash.append(&mut input_bytes.to_vec());
// }

    // let hash = Sha256::digest(&input_to_hash);
}
