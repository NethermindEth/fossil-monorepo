// read from fossil light client to see if it has all the avg_hourly_block_fee that we want
// if so continue,
// if not we need to wait for the fossil light client to catch up,
// maybe we need to update the status and allow PL to recall

// check the batch of hashes,
// for batches that are not available, we need to make a transaction to store it
// once this is done, we make another transaction to hash the batch of hashes

use std::sync::Arc;

use starknet::core::types::TransactionExecutionStatus;
use starknet::core::types::TransactionReceipt::Invoke;
use starknet::providers::Provider;

use crate::services::hashing_service::HashingService;

pub struct HashingProcess {
    hashing_service: Arc<HashingService>,
    required_avg_fees_length: usize,
    hash_batch_size: usize,
}

impl HashingProcess {
    pub fn new(
        hashing_service: HashingService,
        required_avg_fees_length: usize,
        hash_batch_size: usize,
    ) -> Self {
        Self {
            hashing_service: hashing_service.into(),
            required_avg_fees_length,
            hash_batch_size,
        }
    }

    pub async fn run(&self, start_timestamp: u64, end_timestamp: u64) -> Result<(), String> {
        let avg_fees = self
            .hashing_service
            .get_avg_fees_in_range(start_timestamp, end_timestamp)
            .await
            .unwrap();

        // some of the avg_fees are not available, we need to wait for the fossil light client to catch up
        if avg_fees.len() != self.required_avg_fees_length {
            return Err("avg_fees_len is not equal to required_avg_fees_length".to_string());
        }

        // check the batch of hashes
        let mut unavailable_batch_timestamp_hashes = vec![];
        for t in (start_timestamp..end_timestamp).step_by(3600 * self.hash_batch_size) {
            let hash = self.hashing_service.get_hash_stored_avg_fees(t).await;
            if hash.is_err() {
                return Err(hash.err().unwrap().to_string());
            }

            if hash.unwrap() == [0; 8] {
                unavailable_batch_timestamp_hashes.push(t);
            }
        }

        // for batches that are not available, we need to make a transaction to store it
        // hash avg fee and store
        let tasks = unavailable_batch_timestamp_hashes
            .into_iter()
            .map(|t| {
                let hashing_service = self.hashing_service.clone();
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

        // make transaction to hash and store hash on chain
        let mut invoke_tx_tasks = vec![];
        for receipt in receipts {
            if receipt.is_err() {
                return Err(receipt.err().unwrap().to_string());
            }
            let invoke_tx_result = receipt.unwrap();
            let hashing_service = self.hashing_service.clone();

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

        // if everything is successful, we perform batch hash of hash of avg gas fee
        let batch_hash_invoke_res = self
            .hashing_service
            .hash_batched_avg_fees(start_timestamp)
            .await
            .map_err(|e| e.to_string())?;

        // check if it has been successfully stored onchain
        let receipt = self
            .hashing_service
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
