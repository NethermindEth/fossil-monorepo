mod test;

use async_trait::async_trait;
use common::convert_felt_to_f64;
use starknet::{
    accounts::{Account, SingleOwnerAccount},
    core::types::{BlockId, BlockTag, Call, Felt, FunctionCall, InvokeTransactionResult, U256},
    macros::selector,
    providers::{JsonRpcClient, Provider, ProviderError, jsonrpc::HttpTransport},
    signers::LocalWallet,
};

pub struct HashingProvider {
    provider: JsonRpcClient<HttpTransport>,
    fossil_light_client_address: Felt,
    hash_storage_address: Felt,
    account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
}

#[async_trait]
pub trait HashingProviderTrait {
    fn get_provider(&self) -> &JsonRpcClient<HttpTransport>;
    fn get_fossil_light_client_address(&self) -> &Felt;
    fn get_hash_storage_address(&self) -> &Felt;
    async fn get_avg_fees_in_range(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<f64>, ProviderError>;
    async fn get_hash_stored_avg_fees(&self, timestamp: u64) -> Result<[u32; 8], ProviderError>;
    async fn get_hash_batched_avg_fees(
        &self,
        start_timestamp: u64,
    ) -> Result<[u32; 8], ProviderError>;
    async fn hash_avg_fees_and_store(
        &self,
        start_timestamp: u64,
    ) -> Result<InvokeTransactionResult, String>;
    async fn hash_batched_avg_fees(
        &self,
        start_timestamp: u64,
    ) -> Result<InvokeTransactionResult, String>;
}

impl HashingProvider {
    pub fn new(
        provider: JsonRpcClient<HttpTransport>,
        fossil_light_client_address: Felt,
        hash_storage_address: Felt,
        account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    ) -> Self {
        Self {
            provider,
            fossil_light_client_address,
            hash_storage_address,
            account,
        }
    }
}

#[async_trait]
impl HashingProviderTrait for HashingProvider {
    fn get_provider(&self) -> &JsonRpcClient<HttpTransport> {
        &self.provider
    }

    fn get_fossil_light_client_address(&self) -> &Felt {
        &self.fossil_light_client_address
    }

    fn get_hash_storage_address(&self) -> &Felt {
        &self.hash_storage_address
    }

    async fn get_avg_fees_in_range(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<f64>, ProviderError> {
        let mut call_result = self
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
        call_result.remove(0); // the first element is the length of the array, which is not needed by us

        let avg_hourly_fees = call_result
            .iter()
            .map(|fee| convert_felt_to_f64(*fee))
            .collect();

        Ok(avg_hourly_fees)
    }

    async fn get_hash_stored_avg_fees(&self, timestamp: u64) -> Result<[u32; 8], ProviderError> {
        let call_result = self
            .provider
            .call(
                FunctionCall {
                    contract_address: self.hash_storage_address,
                    entry_point_selector: selector!("get_hash_stored_avg_fees"),
                    calldata: vec![Felt::from(timestamp)],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;

        let mut result = [0; 8];
        for i in 0..8 {
            result[i] = U256::from(call_result[i]).low() as u32;
        }

        Ok(result)
    }

    async fn get_hash_batched_avg_fees(
        &self,
        start_timestamp: u64,
    ) -> Result<[u32; 8], ProviderError> {
        let call_result = self
            .provider
            .call(
                FunctionCall {
                    contract_address: self.hash_storage_address,
                    entry_point_selector: selector!("get_hash_stored_batched_avg_fees"),
                    calldata: vec![Felt::from(start_timestamp)],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;

        let mut result = [0; 8];
        for i in 0..8 {
            result[i] = U256::from(call_result[i]).low() as u32;
        }

        Ok(result)
    }

    async fn hash_avg_fees_and_store(
        &self,
        start_timestamp: u64,
    ) -> Result<InvokeTransactionResult, String> {
        let result = self
            .account
            .execute_v3(vec![Call {
                to: self.hash_storage_address,
                selector: selector!("hash_avg_fees_and_store"),
                calldata: vec![Felt::from(start_timestamp)],
            }])
            .send()
            .await
            .map_err(|_| "Error".to_string());

        result
    }

    async fn hash_batched_avg_fees(
        &self,
        start_timestamp: u64,
    ) -> Result<InvokeTransactionResult, String> {
        let result = self
            .account
            .execute_v3(vec![Call {
                to: self.hash_storage_address,
                selector: selector!("hash_batched_avg_fees"),
                calldata: vec![Felt::from(start_timestamp)],
            }])
            .send()
            .await
            .map_err(|_| "Error".to_string());

        result
    }
}