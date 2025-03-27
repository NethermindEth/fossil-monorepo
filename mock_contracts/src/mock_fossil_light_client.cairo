#[starknet::interface]
pub trait IMockFossilLightClient<TContractState> {
    fn get_avg_fee(self: @TContractState, timestamp: u64) -> felt252;
    fn get_avg_fees_in_range(
        self: @TContractState, start_timestamp: u64, end_timestamp: u64,
    ) -> Array<felt252>;

    fn set_avg_fee(ref self: TContractState, timestamp: u64, avg_fee: felt252, data_points: u64);
}

#[starknet::contract]
mod MockFossilLightClient {
    use core::starknet::storage::{
        Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess,
    };
    use super::IMockFossilLightClient;

    #[starknet::storage_node]
    pub struct AvgFees {
        data_points: u64,
        avg_fee: felt252,
    }

    #[storage]
    struct Storage {
        avg_fees: Map<u64, AvgFees>,
    }

    const HOUR_IN_SECONDS: u64 = 3600;

    #[abi(embed_v0)]
    impl MockFossilStoreImpl of IMockFossilLightClient<ContractState> {
        fn get_avg_fee(self: @ContractState, timestamp: u64) -> felt252 {
            let curr_state = self.avg_fees.entry(timestamp);
            curr_state.avg_fee.read()
        }

        fn get_avg_fees_in_range(
            self: @ContractState, start_timestamp: u64, end_timestamp: u64,
        ) -> Array<felt252> {
            assert!(
                start_timestamp <= end_timestamp,
                "Start timestamp must be less than or equal to end timestamp",
            );
            assert!(
                start_timestamp % HOUR_IN_SECONDS == 0,
                "Start timestamp must be a multiple of 3600",
            );
            assert!(
                end_timestamp % HOUR_IN_SECONDS == 0, "End timestamp must be a multiple of 3600",
            );

            let mut fees = array![];

            let mut i = start_timestamp;
            while i <= end_timestamp {
                fees.append(self.get_avg_fee(i));
                i += HOUR_IN_SECONDS;
            };
            fees
        }

        fn set_avg_fee(
            ref self: ContractState, timestamp: u64, avg_fee: felt252, data_points: u64,
        ) {
            let mut avg_fees = self.avg_fees.entry(timestamp);
            avg_fees.data_points.write(data_points);
            avg_fees.avg_fee.write(avg_fee);
        }
    }
}
