#[starknet::interface]
pub trait IMockFossilStore<TContractState> {
    fn get_avg_fee(self: @TContractState, timestamp: u64) -> felt252;
    fn get_avg_fees_in_range(
        self: @TContractState, start_timestamp: u64, end_timestamp: u64,
    ) -> Array<felt252>;
}

#[starknet::contract]
mod MockFossilStore {
    use super::mock_get_avg_fee;

    #[storage]
    struct Storage {}

    #[abi(embed_v0)]
    impl MockFossilStore of super::IMockFossilStore<ContractState> {
        fn get_avg_fee(self: @ContractState, timestamp: u64) -> felt252 {
            0
        }

        fn get_avg_fees_in_range(
            self: @ContractState, start_timestamp: u64, end_timestamp: u64,
        ) -> Array<felt252> {
            mock_get_avg_fee()
        }
    }
}

fn mock_get_avg_fee() -> Array<felt252> {
    let avg_fees = array![];
    avg_fees
}

