// read from fossil light client to see if it has all the avg_hourly_block_fee that we want
// if so continue,
// if not we need to wait for the fossil light client to catch up,
// maybe we need to update the status and allow PL to recall

// check the batch of hashes,
// for batches that are not available, we need to make a transaction to store it
// once this is done, we make another transaction to hash the batch of hashes

use starknet::{
    core::types::{BlockId, BlockTag, Felt, FunctionCall, U256},
    macros::selector,
    providers::{JsonRpcClient, Provider, ProviderError, jsonrpc::HttpTransport},
};

// TODO: to import from pitchlake-coprocessor repo
fn convert_felt_to_f64(input: Felt) -> f64 {
    let input_u256 = U256::from(input);

    const TWO_POW_128: f64 = 340282366920938463463374607431768211456.0;
    let decimal = input_u256.low() as f64 / TWO_POW_128;
    input_u256.high() as f64 + decimal
}

pub struct HashingService {
    provider: JsonRpcClient<HttpTransport>,
    fossil_light_client_address: Felt,
    hash_storage_address: Felt,
}

impl HashingService {
    pub fn new(
        provider: JsonRpcClient<HttpTransport>,
        fossil_light_client_address: Felt,
        hash_storage_address: Felt,
    ) -> Self {
        Self {
            provider,
            fossil_light_client_address,
            hash_storage_address,
        }
    }

    pub async fn get_avg_fees_in_range(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<f64>, ProviderError> {
        let call_result = self
            .provider
            .call(
                FunctionCall {
                    contract_address: self.fossil_light_client_address,
                    entry_point_selector: selector!("get_avg_fees_in_range"),
                    calldata: vec![Felt::from(start_timestamp), Felt::from(end_timestamp)],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;

        let avg_hourly_fees = call_result
            .iter()
            .map(|fee| convert_felt_to_f64(*fee))
            .collect();

        Ok(avg_hourly_fees)
    }
}
