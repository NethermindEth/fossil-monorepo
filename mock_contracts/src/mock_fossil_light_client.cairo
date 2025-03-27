#[starknet::interface]
pub trait IMockFossilLightClient<TContractState> {
    fn get_avg_fee(self: @TContractState, timestamp: u64) -> felt252;
    fn get_avg_fees_in_range(
        self: @TContractState, start_timestamp: u64, end_timestamp: u64,
    ) -> Array<felt252>;

    fn set_avg_fee(ref self: TContractState, timestamp: u64, avg_fee: felt252);
}

#[starknet::contract]
mod MockFossilLightClient {
    use core::starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};

    #[storage]
    struct Storage {}
}