#[cfg(test)]
mod tests {
    use starknet::{
        accounts::{ExecutionEncoding, SingleOwnerAccount},
        core::{
            chain_id,
            types::{BlockId, BlockTag, Felt},
        },
        providers::{JsonRpcClient, Url, jsonrpc::HttpTransport},
        signers::{LocalWallet, SigningKey},
    };

    use crate::hashing::{HashingProvider, HashingProviderTrait};
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
        let account_address = env::var("STARKNET_ACCOUNT_ADDRESS").unwrap();
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

        let hashing = HashingProvider::new(
            provider,
            fossil_light_client_address,
            hash_storage_address,
            account,
        );

        hashing
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

    #[tokio::test]
    async fn test_docker_compose() {
        // start_docker_compose().await;
        let docker_compose_started =
            test_components::StartDockerCompose::start_docker_compose().await;
        assert!(docker_compose_started, "starknet-devnet failed to start");
        let provider = setup();
    }
}
