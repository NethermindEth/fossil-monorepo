#[starknet::interface]
pub trait IFossilMinimalAvgFeeStore<TContractState> {
    fn get_avg_fee(self: @TContractState, timestamp: u64) -> felt252;
    fn get_avg_fees_in_range(
        self: @TContractState, start_timestamp: u64, end_timestamp: u64,
    ) -> Array<felt252>;
}
