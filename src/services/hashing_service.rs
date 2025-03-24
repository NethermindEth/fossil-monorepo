use common::convert_felt_to_f64;
use starknet::{
    accounts::{Account, SingleOwnerAccount},
    core::types::{BlockId, BlockTag, Call, Felt, FunctionCall, InvokeTransactionResult, U256},
    macros::selector,
    providers::{JsonRpcClient, Provider, ProviderError, jsonrpc::HttpTransport},
    signers::LocalWallet,
};

pub struct HashingService {
    provider: JsonRpcClient<HttpTransport>,
    fossil_light_client_address: Felt,
    hash_storage_address: Felt,
    account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
}

impl HashingService {
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

    pub fn get_provider(&self) -> &JsonRpcClient<HttpTransport> {
        &self.provider
    }

    pub fn get_fossil_light_client_address(&self) -> &Felt {
        &self.fossil_light_client_address
    }

    pub async fn get_avg_fees_in_range(
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

    pub async fn get_hash_stored_avg_fees(
        &self,
        timestamp: u64,
    ) -> Result<[u32; 8], ProviderError> {
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

    pub async fn get_hash_batched_avg_fees(
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

    pub async fn hash_avg_fees_and_store(
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

    pub async fn hash_batched_avg_fees(
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

#[cfg(test)]
mod tests {
    use starknet::{
        accounts::ExecutionEncoding, core::chain_id, providers::Url, signers::SigningKey,
    };

    use super::*;

    fn setup() -> HashingService {
        let provider = JsonRpcClient::new(HttpTransport::new(
            // Url::parse("https://rpc.starknet-testnet.lava.build:443").unwrap(),
            // Url::parse("https://starknet-sepolia.public.blastapi.io").unwrap(),
            Url::parse("https://free-rpc.nethermind.io/sepolia-juno").unwrap(),
        ));
        let fossil_light_client_address =
            Felt::from_hex("0x01710d5f515a17943f439c0a5ba4483d44bac0d2b04f5345639c222debc80b2c")
                .unwrap();
        let hash_storage_address =
            Felt::from_hex("0x04807b7b062a49359e6984352590fbc1b285e79f677a8c40b758fd69d5232a89")
                .unwrap();
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex("0xa").unwrap(),
        ));
        let signer_address = Felt::from_hex("0x1").unwrap();
        let mut account = SingleOwnerAccount::new(
            provider.clone(),
            signer,
            signer_address,
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        );

        // `SingleOwnerAccount` defaults to checking nonce and estimating fees against the latest
        // block. Optionally change the target block to pending with the following line:
        account.set_block_id(BlockId::Tag(BlockTag::Pending));

        let hashing_service = HashingService::new(
            provider,
            fossil_light_client_address,
            hash_storage_address,
            account,
        );

        hashing_service
    }

    #[ignore = "calling actual rpc node"]
    #[tokio::test]
    async fn should_retrieve_avg_fees_in_range() {
        let hashing_service = setup();

        let avg_fees = hashing_service
            // .get_avg_fees_in_range(1739304000, 1739307600)
            .get_avg_fees_in_range(1734843600, 1742533200)
            .await
            .unwrap();

        println!("avg_fees_len: {:?}", avg_fees.len());
        println!("{:?}", avg_fees);
    }

    #[ignore = "calling actual rpc node"]
    #[tokio::test]
    async fn should_get_hash_stored_avg_fees() {
        let hashing_service = setup();

        let hash = hashing_service
            .get_hash_stored_avg_fees(1734843600)
            .await
            .unwrap();

        println!("{:?}", hash);
    }

    #[ignore = "calling actual rpc node"]
    #[tokio::test]
    async fn should_get_hash_batched_avg_fees() {
        let hashing_service = setup();

        let hash = hashing_service
            .get_hash_batched_avg_fees(1734843600)
            .await
            .unwrap();

        println!("{:?}", hash);
    }
}
