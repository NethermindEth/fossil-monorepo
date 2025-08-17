use async_trait::async_trait;
#[cfg(feature = "proof-composition")]
use coprocessor_common::convert_felt_to_f64;
use eyre::{Result, eyre};
use starknet::{
    accounts::{Account, SingleOwnerAccount},
    core::types::{BlockId, BlockTag, Call, Felt, FunctionCall, InvokeTransactionResult, U256},
    macros::selector,
    providers::{JsonRpcClient, Provider, ProviderError, jsonrpc::HttpTransport},
    signers::LocalWallet,
};

#[cfg(not(feature = "proof-composition"))]
pub fn convert_felt_to_f64(felt: Felt) -> f64 {
    // Simple fallback implementation when proof-composition is disabled
    let u256_value = U256::from(felt);
    let high_bits = (u256_value.high() as f64) * 2_f64.powi(128);
    let low_bits = u256_value.low() as f64;
    high_bits + low_bits
}

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
    async fn get_avg_fees_in_range_as_felt(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<Felt>, ProviderError>;
    async fn get_hash_stored_avg_fees(&self, timestamp: u64) -> Result<[u32; 8], ProviderError>;
    async fn get_hash_batched_avg_fees(
        &self,
        start_timestamp: u64,
    ) -> Result<[u32; 8], ProviderError>;
    async fn hash_avg_fees_and_store(
        &self,
        start_timestamp: u64,
    ) -> Result<InvokeTransactionResult>;
    async fn hash_batched_avg_fees(&self, start_timestamp: u64) -> Result<InvokeTransactionResult>;
}

impl HashingProvider {
    pub const fn new(
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

    /// Creates a new `HashingProvider` instance from environment variables.
    ///
    /// Requires the following environment variables to be set:
    /// - `RPC_URL`: URL for the Starknet RPC provider
    /// - `FOSSIL_LIGHT_CLIENT_ADDRESS`: Address of the fossil light client contract
    /// - `HASH_STORAGE_ADDRESS`: Address of the hash storage contract
    /// - `STARKNET_PRIVATE_KEY`: Private key for the Starknet account
    /// - `STARKNET_ACCOUNT`: Address of the Starknet account
    ///
    /// # Returns
    /// A Result containing the `HashingProvider` or an error
    pub fn from_env() -> Result<Self> {
        use starknet::{
            accounts::ExecutionEncoding,
            core::chain_id,
            signers::{LocalWallet, SigningKey},
        };
        use std::env;
        use url::Url;

        // Load environment variables
        let rpc_url =
            env::var("RPC_URL").map_err(|_| eyre!("RPC_URL environment variable is not set"))?;

        let fossil_light_client_address = env::var("FOSSIL_LIGHT_CLIENT_ADDRESS")
            .map_err(|_| eyre!("FOSSIL_LIGHT_CLIENT_ADDRESS environment variable is not set"))?;

        let hash_storage_address = env::var("HASH_STORAGE_ADDRESS")
            .map_err(|_| eyre!("HASH_STORAGE_ADDRESS environment variable is not set"))?;

        let private_key = env::var("STARKNET_PRIVATE_KEY")
            .map_err(|_| eyre!("STARKNET_PRIVATE_KEY environment variable is not set"))?;

        let account_address = env::var("STARKNET_ACCOUNT")
            .map_err(|_| eyre!("STARKNET_ACCOUNT environment variable is not set"))?;

        // Create the provider
        let provider = JsonRpcClient::new(HttpTransport::new(
            Url::parse(&rpc_url).map_err(|e| eyre!("Failed to parse RPC URL: {}", e))?,
        ));

        // Convert addresses to Felt
        let fossil_light_client_felt = Felt::from_hex(&fossil_light_client_address)
            .map_err(|e| eyre!("Invalid FOSSIL_LIGHT_CLIENT_ADDRESS: {}", e))?;

        let hash_storage_felt = Felt::from_hex(&hash_storage_address)
            .map_err(|e| eyre!("Invalid HASH_STORAGE_ADDRESS: {}", e))?;

        // Create the signer from private key
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex(&private_key)
                .map_err(|e| eyre!("Invalid STARKNET_PRIVATE_KEY: {}", e))?,
        ));

        // Create account address from hex
        let signer_address = Felt::from_hex(&account_address)
            .map_err(|e| eyre!("Invalid STARKNET_ACCOUNT: {}", e))?;

        // Create the account
        let mut account = SingleOwnerAccount::new(
            provider.clone(),
            signer,
            signer_address,
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        );

        // Set block ID to pending for better transaction handling
        account.set_block_id(BlockId::Tag(BlockTag::Pending));

        // Create and return the HashingProvider
        Ok(Self::new(
            provider,
            fossil_light_client_felt,
            hash_storage_felt,
            account,
        ))
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
        let call_result = self
            .get_avg_fees_in_range_as_felt(start_timestamp, end_timestamp)
            .await?;

        let avg_hourly_fees = call_result
            .iter()
            .map(|fee| convert_felt_to_f64(*fee))
            .collect();

        Ok(avg_hourly_fees)
    }

    async fn get_avg_fees_in_range_as_felt(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<Felt>, ProviderError> {
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

        Ok(call_result)
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
    ) -> Result<InvokeTransactionResult> {
        self.account
            .execute_v3(vec![Call {
                to: self.hash_storage_address,
                selector: selector!("hash_avg_fees_and_store"),
                calldata: vec![Felt::from(start_timestamp)],
            }])
            .send()
            .await
            .map_err(|e| eyre!("Failed to hash and store average fees: {}", e))
    }

    async fn hash_batched_avg_fees(&self, start_timestamp: u64) -> Result<InvokeTransactionResult> {
        self.account
            .execute_v3(vec![Call {
                to: self.hash_storage_address,
                selector: selector!("hash_batched_avg_fees"),
                calldata: vec![Felt::from(start_timestamp)],
            }])
            .send()
            .await
            .map_err(|e| eyre!("Failed to hash batched average fees: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use starknet::{
        accounts::ExecutionEncoding, core::chain_id, providers::Url, signers::SigningKey,
    };

    use super::*;
    use dotenv::dotenv;
    use std::env;

    fn setup() -> HashingProvider {
        dotenv().ok();

        let provider = JsonRpcClient::new(HttpTransport::new(
            Url::parse(&env::var("RPC_URL").unwrap()).unwrap(),
        ));
        let fossil_light_client_address =
            Felt::from_hex(&env::var("FOSSIL_LIGHT_CLIENT_ADDRESS").unwrap()).unwrap();
        let hash_storage_address =
            Felt::from_hex(&env::var("HASH_STORAGE_ADDRESS").unwrap()).unwrap();

        let private_key = env::var("STARKNET_PRIVATE_KEY").unwrap();
        let account_address = env::var("STARKNET_ACCOUNT").unwrap();
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex(&private_key).unwrap(),
        ));
        let signer_address = Felt::from_hex(&account_address).unwrap();
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

        HashingProvider::new(
            provider,
            fossil_light_client_address,
            hash_storage_address,
            account,
        )
    }

    #[ignore = "calling actual rpc node"]
    #[tokio::test]
    async fn should_retrieve_avg_fees_in_range() {
        let hashing = setup();

        let avg_fees = hashing
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
        let hashing = setup();

        let hash = hashing
            .get_hash_stored_avg_fees(1739307600)
            // .get_hash_stored_avg_fees(1734843600)
            .await
            .unwrap();

        println!("{:?}", hash);
    }

    #[ignore = "calling actual rpc node"]
    #[tokio::test]
    async fn should_get_hash_batched_avg_fees() {
        let hashing = setup();

        let hash = hashing.get_hash_batched_avg_fees(1734843600).await.unwrap();

        println!("{:?}", hash);
    }

    #[ignore = "calling actual rpc node"]
    #[tokio::test]
    async fn should_hash_avg_fees_and_store() {
        let hashing = setup();

        let result = hashing.hash_avg_fees_and_store(1739307600).await;
        println!("tx hash: {:?}", result.unwrap().transaction_hash);
    }
}
