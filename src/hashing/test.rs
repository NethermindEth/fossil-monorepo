#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use starknet::{
        core::types::{Felt, InvokeTransactionResult},
        providers::{JsonRpcClient, ProviderError, jsonrpc::HttpTransport},
    };

    use crate::{hashing::HashingProcess, services::hashing_service::HashingServiceTrait};

    struct MockHashingService {
        avg_fees: Vec<f64>,
        hash_stored_avg_fees: [u32; 8],
        hash_batched_avg_fee: [u32; 8],
    }

    impl MockHashingService {
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

        // pub fn set_hash_stored_avg_fees(&mut self, hash_stored_avg_fees: [u32; 8]) {
        //     self.hash_stored_avg_fees = hash_stored_avg_fees;
        // }

        pub fn set_hash_batched_avg_fee(&mut self, hash_batched_avg_fee: [u32; 8]) {
            self.hash_batched_avg_fee = hash_batched_avg_fee;
        }
    }

    #[async_trait]
    impl HashingServiceTrait for MockHashingService {
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

    fn setup() -> HashingProcess<MockHashingService> {
        let hashing_service = MockHashingService::new();
        let process =
            HashingProcess::new(hashing_service, REQUIRED_AVG_FEES_LENGTH, HASH_BATCH_SIZE);

        process
    }

    #[tokio::test]
    async fn should_fail_if_check_avg_fees_availability_not_equals_to_required_avg_fees_length() {
        let process = setup();

        let res = process.check_avg_fees_availability(0, 0).await;
        assert!(
            res.err().unwrap()
                == "avg_fees_len is not equal to required_avg_fees_length".to_string()
        );
    }

    #[tokio::test]
    async fn should_return_ok_if_check_avg_fees_availability_equals_to_required_avg_fees_length() {
        let mut process = setup();

        Arc::get_mut(&mut process.hashing_service)
            .unwrap()
            .set_avg_fees(vec![1.0; REQUIRED_AVG_FEES_LENGTH]);
        let res = process.check_avg_fees_availability(0, 0).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn should_get_unavailable_batch_timestamp_hashes() {
        let process = setup();

        let res = process
            .get_unavailable_batch_timestamp_hashes(0, (2 * 3600 * HASH_BATCH_SIZE as u64) - 1) //first and second batch
            .await;
        assert_eq!(res.unwrap(), vec![0, 3600 * HASH_BATCH_SIZE as u64]);
    }

    #[tokio::test]
    async fn should_return_false_if_batch_hash_avg_fees_is_not_available() {
        let process = setup();

        let res = process.is_batch_hash_avg_fees_available(0).await;
        assert!(res.unwrap() == false);
    }

    #[tokio::test]
    async fn should_return_true_if_batch_hash_avg_fees_is_available() {
        let mut process = setup();

        Arc::get_mut(&mut process.hashing_service)
            .unwrap()
            .set_hash_batched_avg_fee([1; 8]);

        let res = process.is_batch_hash_avg_fees_available(0).await;
        assert!(res.unwrap());
    }
}
