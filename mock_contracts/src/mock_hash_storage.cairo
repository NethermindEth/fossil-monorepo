#[starknet::interface]
pub trait IMockHashStorage<TContractState> {
    fn set_fossil_store(ref self: TContractState, fossil_store: starknet::ContractAddress);
    fn get_fossil_store(self: @TContractState) -> starknet::ContractAddress;
    fn hash_avg_fees_and_store(ref self: TContractState, start_timestamp: u64);
    fn get_hash_stored_avg_fees(self: @TContractState, timestamp: u64) -> [u32; 8];
    fn hash_batched_avg_fees(ref self: TContractState, start_timestamp: u64);
    fn get_hash_stored_batched_avg_fees(self: @TContractState, timestamp: u64) -> [u32; 8];

    fn set_hash_stored_avg_fees(ref self: TContractState, timestamp: u64, hash: [u32; 8]);
    fn set_hash_stored_batched_avg_fees(ref self: TContractState, timestamp: u64, hash: [u32; 8]);
}


#[starknet::contract]
mod MockHashStorage {
    use core::starknet::storage::{
        Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess,
    };
    use crate::mock_fossil_light_client::IMockFossilLightClientDispatcher;

    #[storage]
    struct Storage {
        fossil_store: IMockFossilLightClientDispatcher,
        hash_stored_avg_fees: Map<u64, [u32; 8]>, // hash of 180 avg fees
        hash_batched_avg_fees: Map<u64, [u32; 8]> // hash of hash of 180 avg fees
    }

    #[abi(embed_v0)]
    impl MockHashStorageImpl of super::IMockHashStorage<ContractState> {
        fn set_fossil_store(ref self: ContractState, fossil_store: starknet::ContractAddress) {
            self
                .fossil_store
                .write(IMockFossilLightClientDispatcher { contract_address: fossil_store });
        }

        fn get_fossil_store(self: @ContractState) -> starknet::ContractAddress {
            self.fossil_store.read().contract_address
        }


        // hashing 180 avg fees
        fn hash_avg_fees_and_store(ref self: ContractState, start_timestamp: u64) {}

        fn get_hash_stored_avg_fees(self: @ContractState, timestamp: u64) -> [u32; 8] {
            self.hash_stored_avg_fees.entry(timestamp).read()
        }

        fn hash_batched_avg_fees(ref self: ContractState, start_timestamp: u64) {}

        fn get_hash_stored_batched_avg_fees(self: @ContractState, timestamp: u64) -> [u32; 8] {
            self.hash_batched_avg_fees.entry(timestamp).read()
        }

        fn set_hash_stored_avg_fees(ref self: ContractState, timestamp: u64, hash: [u32; 8]) {
            self.hash_stored_avg_fees.entry(timestamp).write(hash);
        }

        fn set_hash_stored_batched_avg_fees(
            ref self: ContractState, timestamp: u64, hash: [u32; 8],
        ) {
            self.hash_batched_avg_fees.entry(timestamp).write(hash);
        }
    }
}
