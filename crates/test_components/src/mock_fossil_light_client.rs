use starknet::{
    accounts::{Account, SingleOwnerAccount},
    core::types::{BlockId, BlockTag, Call, Felt, FunctionCall, TransactionExecutionStatus},
    macros::selector,
    providers::{JsonRpcClient, Provider, jsonrpc::HttpTransport},
    signers::LocalWallet,
};

pub struct MockFossilLightClient {
    pub contract_address: String,
    pub provider: JsonRpcClient<HttpTransport>,
    pub account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
}

impl MockFossilLightClient {
    pub fn new(
        contract_address: String,
        provider: JsonRpcClient<HttpTransport>,
        account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    ) -> Self {
        Self {
            contract_address,
            provider,
            account,
        }
    }

    pub async fn set_avg_fee(&self, timestamp: u64, avg_fee: u64, data_points: u64) {
        let invoke_result = self
            .account
            .execute_v3(vec![Call {
                to: Felt::from_hex(&self.contract_address).unwrap(),
                selector: selector!("set_avg_fee"),
                calldata: vec![
                    Felt::from(timestamp),
                    Felt::from(avg_fee),
                    Felt::from(data_points),
                ],
            }])
            .send()
            .await
            .unwrap();

        let result = self
            .provider
            .get_transaction_receipt(invoke_result.transaction_hash)
            .await
            .unwrap();

        assert!(result.receipt.execution_result().status() != TransactionExecutionStatus::Reverted);
    }
}
