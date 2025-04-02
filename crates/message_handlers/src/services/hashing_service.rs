use std::sync::Arc;

use starknet::core::types::TransactionExecutionStatus;
use starknet::core::types::TransactionReceipt::Invoke;
use starknet::providers::Provider;

use crate::hashing::HashingProviderTrait;
use std::marker::{Send, Sync};

pub struct HashingService<T: HashingProviderTrait + Sync + Send + 'static> {
    hashing_provider: Arc<T>,
    required_avg_fees_length: usize,
    hash_batch_size: usize,
}

impl<T: HashingProviderTrait + Sync + Send + 'static> HashingService<T> {
    pub fn new(
        hashing_service: T,
        required_avg_fees_length: usize,
        hash_batch_size: usize,
    ) -> Self {
        Self {
            hashing_provider: hashing_service.into(),
            required_avg_fees_length,
            hash_batch_size,
        }
    }

    pub async fn run(&self, start_timestamp: u64) -> Result<(), String> {
        let end_timestamp = start_timestamp + 3600 * (self.required_avg_fees_length as u64 - 1);
        self.check_avg_fees_availability(start_timestamp, end_timestamp)
            .await?;
        let unavailable_batch_timestamp_hashes = self
            .get_unavailable_batch_timestamp_hashes(start_timestamp, end_timestamp)
            .await?;

        if unavailable_batch_timestamp_hashes.len() > 0 {
            self.hash_and_store_avg_fees_onchain(unavailable_batch_timestamp_hashes)
                .await?;
        }

        if !self
            .is_batch_hash_avg_fees_available(start_timestamp)
            .await?
        {
            self.hash_batch_avg_fees_onchain(start_timestamp).await?;
        }

        Ok(())
    }

    async fn check_avg_fees_availability(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<(), String> {
        let avg_fees = self
            .hashing_provider
            .get_avg_fees_in_range(start_timestamp, end_timestamp)
            .await
            .map_err(|e| e.to_string())?;

        if avg_fees.len() != self.required_avg_fees_length {
            return Err("avg_fees_len is not equal to required_avg_fees_length".to_string());
        }
        Ok(())
    }

    async fn get_unavailable_batch_timestamp_hashes(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<u64>, String> {
        let mut unavailable_batch_timestamp_hashes = vec![];
        for t in (start_timestamp..end_timestamp).step_by(3600 * self.hash_batch_size) {
            let hash = self.hashing_provider.get_hash_stored_avg_fees(t).await;
            if hash.is_err() {
                return Err(hash.err().unwrap().to_string());
            }

            if hash.unwrap() == [0; 8] {
                unavailable_batch_timestamp_hashes.push(t);
            }
        }

        Ok(unavailable_batch_timestamp_hashes)
    }

    // for batches that are not available, we need to make a transaction to store it
    // hash avg fee and store
    async fn hash_and_store_avg_fees_onchain(
        &self,
        unavailable_batch_timestamp_hashes: Vec<u64>,
    ) -> Result<(), String> {
        let tasks = unavailable_batch_timestamp_hashes
            .into_iter()
            .map(|t| {
                let hashing_service = self.hashing_provider.clone();
                tokio::task::spawn(async move {
                    let res = hashing_service.hash_avg_fees_and_store(t).await;
                    res
                })
            })
            .collect::<Vec<_>>();

        let mut receipts = vec![];
        for task in tasks {
            let receipt = task.await.map_err(|e| e.to_string())?;
            receipts.push(receipt);
        }

        let mut invoke_tx_tasks = vec![];
        for receipt in receipts {
            if receipt.is_err() {
                return Err(receipt.err().unwrap().to_string());
            }
            let invoke_tx_result = receipt.unwrap();
            let hashing_service = self.hashing_provider.clone();

            let task = tokio::task::spawn(async move {
                hashing_service
                    .get_provider()
                    .get_transaction_receipt(invoke_tx_result.transaction_hash)
                    .await
            });
            invoke_tx_tasks.push(task);
        }

        // check if the invocation is successful and has been stored onchain
        let mut invoke_tx_results = vec![];
        for task in invoke_tx_tasks {
            let res = task.await.map_err(|e| e.to_string())?;
            invoke_tx_results.push(res);
        }

        for invoke_tx_result in invoke_tx_results {
            if invoke_tx_result.is_err() {
                return Err(invoke_tx_result.err().unwrap().to_string());
            }
            if let Invoke(invoke_receipt) = invoke_tx_result.unwrap().receipt {
                if invoke_receipt.execution_result.status() == TransactionExecutionStatus::Reverted
                {
                    return Err("invoke reverted".to_string());
                }
            }
        }

        Ok(())
    }

    async fn is_batch_hash_avg_fees_available(&self, start_timestamp: u64) -> Result<bool, String> {
        let hash = self
            .hashing_provider
            .get_hash_batched_avg_fees(start_timestamp)
            .await
            .map_err(|e| e.to_string())?;

        Ok(hash != [0; 8])
    }

    async fn hash_batch_avg_fees_onchain(&self, start_timestamp: u64) -> Result<(), String> {
        // if everything is successful, we perform batch hash of hash of avg gas fee
        let batch_hash_invoke_res = self
            .hashing_provider
            .hash_batched_avg_fees(start_timestamp)
            .await
            .map_err(|e| e.to_string())?;

        // check if it has been successfully stored onchain
        let receipt = self
            .hashing_provider
            .get_provider()
            .get_transaction_receipt(batch_hash_invoke_res.transaction_hash)
            .await
            .map_err(|e| e.to_string())?;

        if receipt.receipt.execution_result().status() == TransactionExecutionStatus::Reverted {
            return Err("batch hash reverted".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use reqwest::Url;
    use starknet::{
        accounts::{ExecutionEncoding, SingleOwnerAccount},
        core::{
            chain_id,
            types::{Felt, InvokeTransactionResult},
        },
        providers::{JsonRpcClient, ProviderError, jsonrpc::HttpTransport},
        signers::{LocalWallet, SigningKey},
    };
    use test_components::{
        LOCALHOST_FOSSIL_LIGHT_CLIENT_ADDRESS, LOCALHOST_HASH_STORAGE_ADDRESS, LOCALHOST_RPC_URL,
        LOCALHOST_STARKNET_ACCOUNT_ADDRESS, LOCALHOST_STARKNET_PRIVATE_KEY, StartDockerCompose,
    };

    use crate::hashing::{HashingProvider, HashingProviderTrait};

    use super::HashingService;

    // use crate::{hashing::HashingProcess, services::hashing_service::HashingServiceTrait};

    struct MockHashingProvider {
        avg_fees: Vec<f64>,
        hash_stored_avg_fees: [u32; 8],
        hash_batched_avg_fee: [u32; 8],
    }

    impl MockHashingProvider {
        pub fn new() -> Self {
            Self {
                avg_fees: vec![],
                hash_stored_avg_fees: [0; 8],
                hash_batched_avg_fee: [0; 8],
            }
        }

        pub fn set_avg_fees(&mut self, avg_fees: Vec<f64>) {
            self.avg_fees = avg_fees;
        }

        pub fn set_hash_batched_avg_fee(&mut self, hash_batched_avg_fee: [u32; 8]) {
            self.hash_batched_avg_fee = hash_batched_avg_fee;
        }
    }

    #[async_trait]
    impl HashingProviderTrait for MockHashingProvider {
        fn get_provider(&self) -> &JsonRpcClient<HttpTransport> {
            todo!()
        }

        fn get_fossil_light_client_address(&self) -> &Felt {
            todo!()
        }

        fn get_hash_storage_address(&self) -> &Felt {
            todo!()
        }

        async fn get_avg_fees_in_range(
            &self,
            _start_timestamp: u64,
            _end_timestamp: u64,
        ) -> Result<Vec<f64>, ProviderError> {
            Ok(self.avg_fees.clone())
        }

        async fn get_hash_stored_avg_fees(
            &self,
            _timestamp: u64,
        ) -> Result<[u32; 8], ProviderError> {
            Ok(self.hash_stored_avg_fees)
        }

        async fn get_hash_batched_avg_fees(
            &self,
            _start_timestamp: u64,
        ) -> Result<[u32; 8], ProviderError> {
            Ok(self.hash_batched_avg_fee)
        }

        async fn hash_avg_fees_and_store(
            &self,
            _start_timestamp: u64,
        ) -> Result<InvokeTransactionResult, String> {
            todo!()
        }

        async fn hash_batched_avg_fees(
            &self,
            _start_timestamp: u64,
        ) -> Result<InvokeTransactionResult, String> {
            todo!()
        }
    }

    const REQUIRED_AVG_FEES_LENGTH: usize = 10;
    const HASH_BATCH_SIZE: usize = 10;

    fn setup<T: HashingProviderTrait + Sync + Send + 'static>(
        hashing_provider: T,
    ) -> HashingService<T> {
        let service =
            HashingService::new(hashing_provider, REQUIRED_AVG_FEES_LENGTH, HASH_BATCH_SIZE);

        service
    }

    #[tokio::test]
    async fn should_fail_if_check_avg_fees_availability_not_equals_to_required_avg_fees_length() {
        let hashing_provider = MockHashingProvider::new();
        let process = setup(hashing_provider);

        let res = process.check_avg_fees_availability(0, 0).await;
        assert!(
            res.err().unwrap()
                == "avg_fees_len is not equal to required_avg_fees_length".to_string()
        );
    }

    #[tokio::test]
    async fn should_return_ok_if_check_avg_fees_availability_equals_to_required_avg_fees_length() {
        let hashing_provider = MockHashingProvider::new();
        let mut process = setup(hashing_provider);

        Arc::get_mut(&mut process.hashing_provider)
            .unwrap()
            .set_avg_fees(vec![1.0; REQUIRED_AVG_FEES_LENGTH]);
        let res = process.check_avg_fees_availability(0, 0).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn should_get_unavailable_batch_timestamp_hashes() {
        let hashing_provider = MockHashingProvider::new();
        let process = setup(hashing_provider);

        let res = process
            .get_unavailable_batch_timestamp_hashes(0, (2 * 3600 * HASH_BATCH_SIZE as u64) - 1) //first and second batch
            .await;
        assert_eq!(res.unwrap(), vec![0, 3600 * HASH_BATCH_SIZE as u64]);
    }

    #[tokio::test]
    async fn should_return_false_if_batch_hash_avg_fees_is_not_available() {
        let hashing_provider = MockHashingProvider::new();
        let process = setup(hashing_provider);

        let res = process.is_batch_hash_avg_fees_available(0).await;
        assert!(res.unwrap() == false);
    }

    #[tokio::test]
    async fn should_return_true_if_batch_hash_avg_fees_is_available() {
        let hashing_provider = MockHashingProvider::new();
        let mut process = setup(hashing_provider);

        Arc::get_mut(&mut process.hashing_provider)
            .unwrap()
            .set_hash_batched_avg_fee([1; 8]);

        let res = process.is_batch_hash_avg_fees_available(0).await;
        assert!(res.unwrap());
    }

    #[tokio::test]
    async fn should_hash_and_store_avg_fees_onchain() {
        // let docker_compose_started = StartDockerCompose::start_docker_compose().await;
        // assert!(docker_compose_started);

        // let provider =
        //     JsonRpcClient::new(HttpTransport::new(Url::parse(LOCALHOST_RPC_URL).unwrap()));
        // let private_key = LOCALHOST_STARKNET_PRIVATE_KEY;
        // let account_address = LOCALHOST_STARKNET_ACCOUNT_ADDRESS;
        let provider =
        JsonRpcClient::new(HttpTransport::new(Url::parse("https://starknet-sepolia.public.blastapi.io").unwrap()));
        let private_key = "0x065212981820d81714ae3f49ffee54363290eab7a411df1f20d29709a7dbb031";
        let account_address = "0x6307443811a38f62d6eb0908b51f150c91a6c110d8b3a555907f26fbab50d";
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex(&private_key).unwrap(),
        ));
        let signer_address = Felt::from_hex(&account_address).unwrap();
        let account = SingleOwnerAccount::new(
            provider.clone(),
            signer,
            signer_address,
            // Felt::from_hex("0x534e5f5345504f4c4941").unwrap(),
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        );
        let fossil_light_client_address =
            Felt::from_hex(LOCALHOST_FOSSIL_LIGHT_CLIENT_ADDRESS).unwrap();
        let hash_storage_address = Felt::from_hex("0x05b0f3088aa18e506d1b42e606e22bb25bdbfeef48f7821108fecfabd5c3d4a5").unwrap();
        // let hash_storage_address = Felt::from_hex(LOCALHOST_HASH_STORAGE_ADDRESS).unwrap();

        let hashing_provider = HashingProvider::new(
            provider,
            fossil_light_client_address,
            hash_storage_address,
            account,
        );

        let process = setup(hashing_provider);

        // let res = process.check_avg_fees_availability(0, 3600).await;
        // println!("res: {:?}", res);
        let res = process
            .hash_and_store_avg_fees_onchain(vec![0 as u64]) // for test, this doesnt matter
            .await;
        println!("res: {:?}", res);
        assert!(res.is_ok());

        let stop_docker = StartDockerCompose::stop_docker_compose();
        assert!(stop_docker);
    }
}
