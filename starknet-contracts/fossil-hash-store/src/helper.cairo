pub mod Helper {
    use core::sha256::compute_sha256_u32_array;

    fn two_power_u256(n: u32) -> u256 {
        let mut result = 1_u256;
        for _ in 0..n {
            result *= 2;
        }
        result
    }

    pub fn convert_avg_fees_to_u32_array(avg_fees: felt252) -> Array<u32> {
        let mut res = array![];
        let mut avg_fees: u256 = avg_fees.try_into().unwrap();
        for _ in 0..8_u32 {
            let word: u32 = (avg_fees & 0xffffffff_u256).try_into().unwrap();
            res.append(word);
            avg_fees /= two_power_u256(32);
        }
        res
    }

    pub fn hash_of_avg_fees(avg_fees: Array<felt252>) -> [u32; 8] {
        let mut input = array![];
        for fee in avg_fees {
            input.append_span(convert_avg_fees_to_u32_array(fee).span());
        }

        let hash_res = compute_sha256_u32_array(input, 0, 0);
        hash_res
    }

    pub fn hash_of_hash_of_avg_fees(hashes: Array<[u32; 8]>) -> [u32; 8] {
        let mut input = array![];
        for hash in hashes {
            input.append_span(hash.span());
        }

        let hash_res = compute_sha256_u32_array(input, 0, 0);
        hash_res
    }
}
