#[starknet::interface]
pub trait IFossilClient<TContractState> {
    fn fossil_callback(ref self: TContractState, job_request: Span<felt252>, result: Span<felt252>);
}

#[starknet::contract]
pub mod MockPitchLakeClient {
    #[storage]
    struct Storage {}

    #[abi(embed_v0)]
    impl FossilClientImpl of super::IFossilClient<ContractState> {
        fn fossil_callback(
            ref self: ContractState, job_request: Span<felt252>, result: Span<felt252>,
        ) { // Do nothing
        }
    }
}
