#[starknet::interface]
pub trait IPitchLakeVault<TContractState> {
    fn fossil_callback(ref self: TContractState, job_request: Span<felt252>, result: Span<felt252>);
}

#[starknet::contract]
pub mod MockPitchLakeVault {
    #[storage]
    struct Storage {}

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        CallbackEvent: CallbackEvent,
    }

    #[derive(Drop, starknet::Event)]
    struct CallbackEvent {
        job_request: Span<felt252>,
        result: Span<felt252>,
    }

    #[abi(embed_v0)]
    impl FossilClientImpl of super::IPitchLakeVault<ContractState> {
        fn fossil_callback(
            ref self: ContractState, job_request: Span<felt252>, result: Span<felt252>,
        ) {
            self.emit(CallbackEvent { job_request, result });
        }
    }
}
